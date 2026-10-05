use std::{
    io::{self, Error, ErrorKind},
    path::PathBuf,
};

pub fn read_bytes(path: PathBuf) -> Result<String, io::Error> {
    if !path.is_file() {
        return Err(Error::new(ErrorKind::InvalidInput, "path is not a file"));
    }

    Ok("stop complaining".to_owned())
}
