use std::{io, path::Path};

use crate::vault::index::FsIndex;

pub struct Vault {
    index: FsIndex,
}

impl Vault {
    fn open(_path: impl AsRef<Path>) -> Result<Self, io::Error> {
        todo!()
    }

    fn read_file(_id: String) -> Result<(), io::Error> {
        todo!()
    }

    fn create_file(_path: impl AsRef<Path>) -> Result<(), io::Error> {
        todo!()
    }

    fn delete_file(_id: String) -> Result<(), io::Error> {
        todo!()
    }
}
