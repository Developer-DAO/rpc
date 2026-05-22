use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use http_body_util::Limited;

pub const SIZE_LIMIT: usize = 2 * 1024 * 1024;

pub async fn body_size_limiter(request: Request, next: Next) -> Result<Response, String> {
    let (parts, body) = request.into_parts();
    let limited_body = Limited::new(body, SIZE_LIMIT);
    let req = Request::from_parts(parts, Body::new(limited_body));
    Ok(next.run(req).await)
}
