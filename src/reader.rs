use std::io::Read;
use std::path::Path;

use anyhow::Result;
use fastq::{Parser, Record, RecordRefIter};

use crate::{BCLENGTH, CCLENGTH};

pub struct Reader {
    r1: RecordRefIter<Box<dyn Read>>,
    r2: RecordRefIter<Box<dyn Read>>,
}

impl Reader {
    pub fn from_paths<P: AsRef<Path>>(r1: P, r2: P) -> Result<Reader> {
        let (f1, _format) = niffler::from_path(r1)?;
        let (f2, _format) = niffler::from_path(r2)?;

        let p1 = Parser::new(f1);
        let p2 = Parser::new(f2);

        Ok(Reader {
            r1: p1.ref_iter(),
            r2: p2.ref_iter(),
        })
    }

    pub fn read_code(&mut self, cc: &mut [u8], bc: &mut [u8]) -> Option<Result<()>> {
        if let Err(e) = self.r1.advance() {
            return Some(Err(e.into()));
        }

        if let Err(e) = self.r2.advance() {
            return Some(Err(e.into()));
        }

        let read1 = self.r1.get()?;
        let read2 = self.r2.get()?;

        cc.copy_from_slice(&read1.seq()[0..CCLENGTH]);
        bc.copy_from_slice(&read2.seq()[10..][..BCLENGTH]);

        Some(Ok(()))
    }
}
