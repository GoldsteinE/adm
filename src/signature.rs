#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("`X-Hub-Signature-256` header isn't found")]
    HeaderNotFound,
    #[error("`X-Hub-Signature-256` has invalid length")]
    InvalidLength,
    #[error("`X-Hub-Signature-256` must start with `sha256=`")]
    InvalidPrefix,
    #[error("signature must be 64 hex digits")]
    NotHex,
}

#[derive(Debug, Clone)]
pub struct Signature(pub [u8; 32]);

impl Signature {
    pub fn from_headers(headers: &actix_web::http::HeaderMap) -> Result<Self, Error> {
        let sig_b = headers
            .get("X-Hub-Signature-256")
            .ok_or(Error::HeaderNotFound)?
            .as_ref();

        let prefix = b"sha256=";
        let prefix_len = prefix.len();
        if sig_b.len() != 64 + prefix_len {
            return Err(Error::InvalidLength);
        }
        let (sig_prefix, sig_b) = sig_b.split_at(prefix_len);
        if sig_prefix != prefix {
            return Err(Error::InvalidPrefix);
        }

        hex::FromHex::from_hex(sig_b)
            .map(Self)
            .map_err(|_| Error::NotHex)
    }
}
