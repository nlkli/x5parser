use thiserror::Error as ThisError; 
use std::io::Error as StdIoError;
use serde_json::Error as SerdeJsonError;
use tokio::time::error::Elapsed as TokioTimeoutError;
use chromiumoxide::error::CdpError as ChromeDevToolsProtocolError;
use rusqlite::Error as DBError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    ChromeDevToolsProtocol(#[from] ChromeDevToolsProtocolError),

    #[error(transparent)]
    SerdeJson(#[from] SerdeJsonError),

    #[error(transparent)]
    DB(#[from] DBError),

    #[error(transparent)]
    Elapsed(#[from] TokioTimeoutError),

    #[error(transparent)]
    Io(#[from] StdIoError),
}
