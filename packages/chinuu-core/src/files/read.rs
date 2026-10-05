use std::{
    fs,
    io::{self, Error, ErrorKind},
    path::PathBuf,
};

pub fn read_file(path: PathBuf) -> Result<String, io::Error> {
    if !path.is_file() {
        return Err(Error::new(ErrorKind::InvalidInput, "path is not a file"));
    }

    fs::read_to_string(path)
}
