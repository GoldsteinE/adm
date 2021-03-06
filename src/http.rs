use actix_web::{
    dev::Payload, error::ResponseError, http::StatusCode, web::Bytes, FromRequest, HttpRequest,
};
use futures::future::{FutureExt, LocalBoxFuture};
use secstr::SecUtf8;

use crate::signature::{self, Signature};

#[derive(Debug, Clone)]
pub struct Webhook<T>(pub T);

#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("failed parsing signature: {0}")]
    SignatureParseError(#[from] signature::Error),
    #[error("signature doesn't match")]
    InvalidSignature,
    #[error("HMAC key is not specified")]
    NoHmacKey,
    #[error("HMAC key has invalid length")]
    HmacInvalidLength,
    #[error("failed reading request data: {0}")]
    ActixError(#[from] actix_web::Error),
    #[error("invalid JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl From<hmac::crypto_mac::InvalidKeyLength> for WebhookError {
    fn from(_: hmac::crypto_mac::InvalidKeyLength) -> Self {
        Self::HmacInvalidLength
    }
}

impl ResponseError for WebhookError {
    fn status_code(&self) -> StatusCode {
        match self {
            WebhookError::SignatureParseError(_) | WebhookError::JsonError(_) => {
                StatusCode::BAD_REQUEST
            },
            WebhookError::InvalidSignature => StatusCode::FORBIDDEN,
            WebhookError::NoHmacKey | WebhookError::HmacInvalidLength => {
                StatusCode::INTERNAL_SERVER_ERROR
            },
            WebhookError::ActixError(err) => err.as_response_error().status_code(),
        }
    }
}

#[derive(Debug, Default)]
pub struct WebhookConfig {
    pub key: Option<SecUtf8>,
}

impl WebhookConfig {
    pub fn new(key: SecUtf8) -> Self {
        Self { key: Some(key) }
    }
}

impl<T> FromRequest for Webhook<T>
where
    T: serde::de::DeserializeOwned,
{
    type Config = WebhookConfig;
    type Error = WebhookError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(Bytes::from_request(&req, payload).map(
            move |bytes| -> Result<Self, Self::Error> {
                use hmac::{Mac as _, NewMac as _};

                let config = req
                    .app_data::<Self::Config>()
                    .unwrap_or(&WebhookConfig { key: None });

                let bytes = bytes?;
                let actual_signature = {
                    let mut mac = hmac::Hmac::<sha2::Sha256>::new_varkey(
                        config
                            .key
                            .as_ref()
                            .ok_or(WebhookError::NoHmacKey)?
                            .unsecure()
                            .as_bytes(),
                    )?;
                    mac.update(&bytes);
                    mac.finalize()
                };

                let signature = Signature::from_headers(req.headers())?;
                let expected_signature = hmac::crypto_mac::Output::new(signature.0.into());
                if expected_signature != actual_signature {
                    return Err(WebhookError::InvalidSignature);
                }

                Ok(Self(serde_json::from_slice(&bytes)?))
            },
        ))
    }
}
