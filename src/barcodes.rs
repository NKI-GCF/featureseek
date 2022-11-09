use std::fs::File;
use std::io::{Error as IoError, ErrorKind, Write};
use std::path::Path;

use ahash::AHashMap;
use anyhow::Result;
use bktree::BkTree;
use triple_accel::levenshtein::levenshtein_exp;

use crate::{Barcode, BCLENGTH};

fn dist(a: &Barcode, b: &Barcode) -> isize {
    levenshtein_exp(a, b) as isize
}

pub struct Barcodes {
    pub records: Vec<csv::StringRecord>,
    header: csv::StringRecord,
    barcodes: AHashMap<Barcode, usize>,
    bktree: BkTree<Barcode>,
}
pub enum MatchResult {
    NoHit,
    Multiple,
    Unique(usize),
    Dist(usize, isize),
}

impl Barcodes {
    pub fn from_csv<P: AsRef<Path>>(p: P) -> Result<Self> {
        let f = File::open(p)?;
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(true)
            .from_reader(f);

        //TODO check headers for Cellranger compatibility
        let header = reader.headers()?.to_owned();
        if header.as_slice() != "idnamereadpatternsequencefeature_type" {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "Header error: Expected header: id,name,read,pattern,sequence,feature_type",
            )
            .into());
        }

        let mut records = Vec::new();
        let mut barcodes = AHashMap::new();
        for (pos, result) in reader.records().enumerate() {
            let record = result?;

            let barcode = record
                .get(4)
                .ok_or_else(|| IoError::new(
                    ErrorKind::InvalidData,
                    "Expected barcode in column 5",
                ))?
                .as_bytes()
                .try_into()
                .map_err(|_e| {
                    IoError::new(
                        ErrorKind::InvalidData,
                        format!("Barcode length not equal to {}", BCLENGTH),
                    )
                })?;

            records.push(record);
            barcodes.insert(barcode, pos);
        }

        let mut bktree = BkTree::new(dist);
        bktree.insert_all(barcodes.keys().cloned());

        Ok(Barcodes {
            records,
            header,
            barcodes,
            bktree,
        })
    }

    pub fn find(&self, s: &Barcode, approximate: bool) -> MatchResult {
        if let Some(&i) = self.barcodes.get(s.as_slice()) {
            MatchResult::Unique(i)
        } else if approximate {
            let hits = self.bktree.find(s.to_owned(), 2);
            match hits.len() {
                0 => MatchResult::NoHit,
                1 => MatchResult::Dist(*self.barcodes.get(hits[0].0).unwrap(), hits[0].1),
                _ => MatchResult::Multiple,
            }
        } else {
            MatchResult::NoHit
        }
    }

    pub fn write_csv<W: Write, I: IntoIterator<Item = usize>>(&self, w: W, list: I) -> Result<()> {
        let mut writer = csv::Writer::from_writer(w);

        let mut positions: Vec<_> = list.into_iter().collect();
        positions.sort_by_key(|&e| self.records[e].get(0).unwrap());

        writer.write_record(&self.header)?;
        positions
            .into_iter()
            .try_for_each(|pos| writer.write_record(&self.records[pos]))?;
        Ok(())
    }
}
