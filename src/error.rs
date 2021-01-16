use std::{
    io::{
        self
    },
    path::{
        Path,
        PathBuf
    }
};
use tokio::{
    sync::{
        mpsc::{
            error::{
                SendError
            }
        }
    }
};
use bytes::{
    Bytes
};
use url::{
    ParseError
};
use nom::{
    self
};
use quick_error::{
    quick_error
};

// TODO: Попробовать
// - quick-error = "2.0.0"
// - error-chain = "0.12.4"
// - thiserror = "1.0.23"
// - anyhow = "1.0.38"

quick_error!{
    #[derive(Debug)]
    pub enum AppError {
        /// Invalid stream index
        WrongStreamIndex(index: u8) {
            from()
            display(x) -> ("Wrong stream index: {}", index)
        }

        /// IO Error
        IoError(err: std::io::Error) {
            from()
            display(x) -> ("I/O: {}", err)
        }

        /// Chunks receive timeout
        Timeout(err: tokio::time::error::Elapsed) {
            from()
        }

        /// File error
        FileError(filename: PathBuf, err: io::Error) {
            context(path: &'a Path, err: io::Error) -> (path.to_path_buf(), err)
        }

        /// Join error
        JoinError(err: tokio::task::JoinError) {
            from()
        }

        /// Join error
        ReceiverSendError {
            from(SendError<Bytes>)
        }

        /// URL error
        UrlError(err: ParseError) {
            from()
            display(x) -> ("Url error: {}", err)
        }

        /// URL error
        EmptyUrlSegmentsError {
        }

        /// Reqwest Error
        ReqwestError(err: reqwest::Error) {
            from()
            display("Reqwest error: {}", err)
        }

        /// Formatting error
        FormatError {
            from(std::fmt::Error)
        }

        /// M3U error
        M3U8ParseError(err: nom::Err<nom::error::ErrorKind>) {
            from()
            from(err: nom::Err<(&[u8], nom::error::ErrorKind)>) -> (
                match err {
                    nom::Err::Error(e) => {
                        nom::Err::Error(e.1)
                    },
                    nom::Err::Failure(e) => {
                        nom::Err::Failure(e.1)
                    },
                    nom::Err::Incomplete(e) => {
                        nom::Err::Incomplete(e)
                    }
                }
            )
        }

        /// Empty master stream
        MasterStreamIsEmpty {
            display("Master stream is empty")
        }
    }
}

