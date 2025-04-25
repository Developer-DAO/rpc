use super::policy::{Policy, State};
use crate::proxy::errors::HttpClientErrors;
use axum::body::Body;
use hyper::{
    Method,
    Response,
    StatusCode,
    Uri,
    body::{Bytes, Incoming},
    //    client::conn::http1::SendRequest as H1Request,
    header::{HOST, HeaderValue, LOCATION},
};
use hyper_util::rt::TokioIo;
use std::ops::Not;
// if I decide to support cross-origin requests for redirects,
// please be sure to strip any sensitive data contained in headers such as JWT
#[derive(Debug)]
pub struct ProxyClient {
    req: hyper::Request<Bytes>,
    _redirect_policy: Policy,
}

impl ProxyClient {
    pub fn new(req: hyper::Request<Bytes>) -> Self {
        Self {
            req,
            _redirect_policy: Policy::default(),
        }
    }

    pub fn with_policy(req: hyper::Request<Bytes>, policy: Policy) -> Self {
        Self {
            req,
            _redirect_policy: policy,
        }
    }

    pub async fn exec_request(mut self) -> Result<Response<Incoming>, HttpClientErrors> {
        *self.req.uri_mut() = dotenvy::var("SEPOLIA_PROVIDER").unwrap().parse::<hyper::Uri>().unwrap();
        let address = {
            let url = self.req.uri();
            let host = url.host().expect("uri has no host");
            let port = url.port_u16().unwrap_or(80);
            format!("{}:{}", host, port)
        };

        let stream = tokio::net::TcpStream::connect(address).await.unwrap();

        let io = TokioIo::new(stream);

        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await.unwrap();

        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });

        let authority = self.req.uri().authority().unwrap();

        let header_value = HeaderValue::from_str(authority.as_str()).unwrap();

        self.req.headers_mut().insert(HOST, header_value);
        // copy bytes into new request

        let new_request = self.req.clone().map(|r| Body::from(r));
        println!("NEW REQUEST: {:?}\n", &new_request);
        let mut res = sender.send_request(new_request).await?;
        self._redirect_policy.state = self.handle_response(&mut res).await?;

        while let State::Continue = self._redirect_policy.state {
            let new_request = self.req.clone().map(|r| Body::from(r));
            println!("REDIRECT REQUEST: {:?}\n", &new_request);
            res = sender.send_request(new_request).await?;
            self._redirect_policy.state = self.handle_response(&mut res).await?;
        }
        Ok(res)
    }

    fn get_redirect_uri(&self, res: &Response<Incoming>) -> Option<Uri> {
        res.headers()
            .get("Location")
            .map(|v: &HeaderValue| v.to_str().map(|value| Uri::try_from(value).ok()))
            .transpose()
            .ok()
            .flatten()
            .flatten()
            .inspect(|redirect| println!("REDIRECT URL: {redirect}\n"))
    }

    fn is_same_host(&self, other: &hyper::Uri) -> bool {
        self.req.uri().host() == other.host() && self.req.uri().port_u16() == other.port_u16()
    }

    fn swap_location_headers(&mut self, other: &mut Response<Incoming>) {
        let loc_header = &mut self.req.headers_mut().get_mut(LOCATION);

        if let Some(h) = other.headers_mut().get_mut(LOCATION) {
            std::mem::swap(loc_header, &mut Some(h))
        }
    }

    pub async fn handle_response(
        &mut self,
        res: &mut Response<Incoming>,
    ) -> Result<State, super::errors::HttpClientErrors> {
        match self._redirect_policy.apply_redirect_policy() {
            State::Return => Ok(State::Return),
            State::Continue => match self.get_redirect_uri(res) {
                None => Ok(State::Return),
                Some(uri) => match res.status() {
                    StatusCode::MOVED_PERMANENTLY
                    | StatusCode::PERMANENT_REDIRECT
                    | StatusCode::FOUND
                    | StatusCode::TEMPORARY_REDIRECT => {
                        self.follow_redirect(&uri, res).await?;
                        Ok(State::Continue)
                    }
                    StatusCode::SEE_OTHER => {
                        *self.req.method_mut() = Method::GET;
                        *self.req.body_mut() = Bytes::default();
                        self.follow_redirect(&uri, res).await?;
                        Ok(State::Continue)
                    }
                    _ => Ok(State::Return),
                },
            },
        }
    }

    async fn follow_redirect(
        &mut self,
        uri: &Uri,
        res: &mut Response<Incoming>,
    ) -> Result<(), super::errors::HttpClientErrors> {
        if self.is_same_host(uri).not() {
            Err(HttpClientErrors::ExternalHostRedirect)?
        }
        self.swap_location_headers(res);
        Ok(())
    }
}
