use std::{io, path::Path};

use crate::vault::index::FsIndex;

pub struct Vault {
    index: FsIndex,
}

impl Vault {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        todo!()
    }

    pub fn read_file(id: String) -> Result<(), io::Error> {
        todo!()
    }

    pub fn create_file(path: impl AsRef<Path>) -> Result<(), io::Error> {
        todo!()
    }

    pub fn delete_file(id: String) -> Result<(), io::Error> {
        todo!()
    }
}
