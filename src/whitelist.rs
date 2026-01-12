use std::io::{BufRead, BufReader, Error as IoError};
use std::path::Path;

use ahash::AHashSet;
use anyhow::Result;

pub struct Whitelist(AHashSet<Vec<u8>>);

impl Whitelist {
    pub fn from_path<P: AsRef<Path>>(p: P) -> Result<Self> {
        let (f, _format) = niffler::from_path(p.as_ref())?;
        let b = BufReader::new(f);
        let hash = b.split(b'\n').collect::<Result<_, IoError>>()?;

        Ok(Whitelist(hash))
    }

    pub fn contains(&self, v: &[u8]) -> bool {
        self.0.contains(v)
    }
}
