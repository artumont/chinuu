use std::path::Path;

use crate::fileio::{errors::ChinuuFileIoError, types::File};

pub fn create_file(path: impl AsRef<Path>, filename: String) -> Result<(), ChinuuFileIoError> {
    todo!()
}

pub fn write_bytes(file: File, content: Vec<u8>) -> Result<(), ChinuuFileIoError> {
    todo!()
}

pub fn write_string(file: File, content: String) -> Result<(), ChinuuFileIoError> {
    todo!()
}
