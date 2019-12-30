use std::{io, string::FromUtf8Error};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("File does not start with b\"Unity\": `{0:?}`")]
    Magic([u8; 5]),

    #[error("Unrecognized file signature {0:?}")]
    Signature(Vec<u8>),

    #[error("IO error")]
    IO(#[from] io::Error),

    #[error("Invalid UTF-8")]
    UTF8(#[from] FromUtf8Error),
}
