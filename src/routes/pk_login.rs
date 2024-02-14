use super::{
    errors::ApiError,
    types::{Claims, JWT_KEY},
};
use crate::{
    database::{
        errors::ParsingError,
        types::{Role, RELATIONAL_DATABASE},
    },
    eth_rpc::types::Address,
};
use axum::{extract::Query, http::StatusCode, response::IntoResponse, Json};
use jwt_simple::{algorithms::MACLike, reexports::coarsetime::Duration};
use rand::{rngs::ThreadRng, Rng};
use secp256k1::{ecdsa::Signature, Message, PublicKey, Secp256k1};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PkLoginRequest {
    address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PkLoginChallenge {
    activated: bool,
    verificationcode: String,
}

#[tracing::instrument]
pub async fn pk_login_challenge(
    Query(payload): Query<PkLoginRequest>,
) -> Result<impl IntoResponse, ApiError<PkLoginError>> {
    let user = sqlx::query_as!(
        PkLoginChallenge,
        "SELECT verificationCode, activated FROM Customers where wallet = $1",
        &payload.address
    )
    .fetch_optional(RELATIONAL_DATABASE.get().unwrap())
    .await?
    .ok_or_else(|| ApiError::new(PkLoginError::UserNotFound))?;

    if !user.activated {
        Err(ApiError::new(PkLoginError::AccountNotActivated))?
    }

    Ok((
        StatusCode::OK,
        format!(
            "You are signing into D_D RPC. Special Code: {}",
            user.verificationcode
        ),
    )
        .into_response())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PkLoginAuth {
    sig: String,
    pubkey: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PkLoginVerification {
    wallet: String,
    verificationcode: String,
    role: Role,
    email: String,
}

#[tracing::instrument]
pub async fn pk_login_response(
    Json(payload): Json<PkLoginAuth>,
) -> Result<impl IntoResponse, ApiError<PkLoginError>> {
    let db = RELATIONAL_DATABASE.get().unwrap();
    let verification_address = {
        let mut address_hasher = Keccak256::default();
        address_hasher.update(payload.pubkey.as_bytes());
        let hash_raw = address_hasher.finalize();
        let mut hash: [u8; 32] = [0u8; 32];
        hash.copy_from_slice(&hash_raw);
        let hash_string = hex::encode(hash);
        format!("0x{}", &hash_string[hash_string.len() - 41..])
    };
    let user = sqlx::query_as!(PkLoginVerification, 
        r#"SELECT verificationCode, wallet, email, role as "role!: Role" FROM Customers where email = $1"#,
        &verification_address
    ).fetch_optional(db)
    .await?
    .ok_or_else(|| ApiError::new(PkLoginError::UserNotFound))?;
    let sign_in_str = format!(
        "You are signing into D_D RPC. Special Code: {}",
        &user.verificationcode
    );
    let mut hasher = Keccak256::default();
    hasher.update(sign_in_str.as_bytes());
    let res = hasher.finalize();
    let mut hash: [u8; 32] = [0u8; 32];
    hash.copy_from_slice(&res);
    let msg = Message::from_digest(hash);
    let signature = Signature::from_str(&payload.sig)?;
    let pk = PublicKey::from_str(&payload.pubkey)?;
    Secp256k1::new().verify_ecdsa(&msg, &signature, &pk)?;
    // check that the final 20 bytes of signing pubkey is equal to the address we have in the database
    if verification_address != user.wallet {
        Err(ApiError::new(PkLoginError::WrongSigner))?
    }

    let new_code: u32 = ThreadRng::default().gen_range(10000000..99999999);
    sqlx::query!(
        "UPDATE Customers SET activated = true, verificationCode = $1 WHERE email = $2",
        new_code.to_string(),
        &user.email
    )
    .execute(db)
    .await?;

    let user_info = Claims {
        role: user.role,
        email: user.email,
        wallet: Address(user.wallet),
    };
    let claims = jwt_simple::claims::Claims::with_custom_claims(user_info, Duration::from_hours(2));

    let key = JWT_KEY.get().unwrap();
    let jwt = key.authenticate(claims)?;
    Ok((StatusCode::OK, jwt).into_response())
}

#[derive(Debug)]
pub enum PkLoginError {
    AccountNotActivated,
    DatabaseError(sqlx::Error),
    WrongSigner,
    UserNotFound,
    EcdsaError(secp256k1::Error),
    JwtError(jwt_simple::Error),
    AddressParsingError(ParsingError),
}

impl Display for PkLoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PkLoginError::WrongSigner => write!(
                f,
                "The recovered signer is the incorrect pubkey for this account"
            ),
            PkLoginError::DatabaseError(e) => {
                write!(f, "Something went wrong while querying the database: {}", e)
            }
            PkLoginError::AccountNotActivated => write!(f, "This account is yet to be activated"),
            PkLoginError::UserNotFound => write!(f, "User not found!"),
            PkLoginError::EcdsaError(e) => write!(
                f,
                "An error occured while verifying the user's signature: {}",
                e
            ),
            PkLoginError::JwtError(e) => write!(f, "An error occured while creating a JWT: {}", e),
            PkLoginError::AddressParsingError(e) => write!(
                f,
                "An error occured while parsing a string into an address: {}",
                e
            ),
        }
    }
}

impl Error for PkLoginError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PkLoginError::AccountNotActivated => None,
            PkLoginError::DatabaseError(e) => Some(e),
            PkLoginError::WrongSigner => None,
            PkLoginError::UserNotFound => None,
            PkLoginError::EcdsaError(e) => Some(e),
            PkLoginError::JwtError(e) => e.source(),
            PkLoginError::AddressParsingError(e) => Some(e),
        }
    }
}

impl From<sqlx::Error> for ApiError<PkLoginError> {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(PkLoginError::DatabaseError(value))
    }
}

impl From<jwt_simple::Error> for ApiError<PkLoginError> {
    fn from(value: jwt_simple::Error) -> Self {
        ApiError::new(PkLoginError::JwtError(value))
    }
}

impl From<secp256k1::Error> for ApiError<PkLoginError> {
    fn from(value: secp256k1::Error) -> Self {
        ApiError::new(PkLoginError::EcdsaError(value))
    }
}

impl From<ParsingError> for ApiError<PkLoginError> {
    fn from(value: ParsingError) -> Self {
        ApiError::new(PkLoginError::AddressParsingError(value))
    }
}
