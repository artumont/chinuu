use std::{
    fs,
    io::{self, Error, ErrorKind},
    path::PathBuf,
};

pub fn write_file(path: PathBuf, content: &str) -> Result<(), io::Error> {
    if path.is_dir() {
        return Err(Error::new(ErrorKind::InvalidInput, "path is a directory"));
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(path, content)
}
