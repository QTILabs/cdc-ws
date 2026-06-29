use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to connect to the CDC daemon: {0}")]
    GrpcConnect(#[from] tonic::transport::Error),
    #[error("bcrypt hashing failed: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
}

pub type AppResult<T> = Result<T, AppError>;
