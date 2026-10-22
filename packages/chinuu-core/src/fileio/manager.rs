use std::path::Path;

use crate::fileio::{errors::ChinuuFileIoError, types::File};

pub fn delete_file(file: File) -> Result<(), ChinuuFileIoError> {
    todo!()
}

pub fn copy_file(file: File, path: impl AsRef<Path>) -> Result<(), ChinuuFileIoError> {
    todo!()
}

pub fn move_file(file: File, path: impl AsRef<Path>) -> Result<(), ChinuuFileIoError> {
    todo!()
}
