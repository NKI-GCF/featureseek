use std::io::Write;
use std::hash::Hash;

use ahash::AHashMap as HashMap;
use anyhow::Result;
use cli_table::{
    format::{Border, Justify, Separator},
    Cell, Color, Style, Table, TableStruct,
};

use crate::barcodes::Barcodes;
use crate::{CellCode, Barcode, BarcodeRef};

/// Count the barcode (usize references) per cellcode
#[derive(Default)]
pub struct Counts {
    cells: CellCounts<BarcodeRef>,
    ignored: usize,
    multiple: usize,
    nohit: usize,
    not_whitelisted: usize,
    unknown: CellCounts<Barcode>,
}

#[derive(Default)]
struct BarcodeCounts<T>(HashMap<T, usize>);

#[derive(Default)]
pub struct CellCounts<T>(HashMap<CellCode, BarcodeCounts<T>>);

pub struct Summary<'a> {
    barcodes: &'a Barcodes,
    counts: &'a Counts,
}

impl Counts {
    pub fn count_barcode(&mut self, cellcode: CellCode, pos: usize) {
        let cell = self.cells.0.entry(cellcode).or_default();
        cell.count(pos);
    }

    pub fn count_unknown(&mut self, cellcode: CellCode, barcode: Barcode) {
        let cell = self.unknown.0.entry(cellcode).or_default();
        cell.count(barcode);
    }

    pub fn ignored(&mut self) {
        self.ignored += 1;
    }

    pub fn nohit(&mut self) {
        self.nohit += 1;
    }

    pub fn multiple(&mut self) {
        self.multiple += 1;
    }

    pub fn not_whitelisted(&mut self) {
        self.not_whitelisted += 1;
    }
}

impl<T> BarcodeCounts<T> where T: Eq + Hash {

    /// Add a count for the provided barcode
    pub fn count(&mut self, cell_id: T) {
        if let Some(count) = self.0.get_mut(&cell_id) {
            *count += 1;
        } else {
            self.0.insert(cell_id, 1);
        }
    }

    /// Filter the barcodes to those having more than min_reads counts
    pub fn filter_hits(&self, min_reads: usize) -> impl Iterator<Item=(&T, usize)> {
        self.0
            .iter()
            .filter_map(move |(id, count)| {
                if *count > min_reads {
                    Some((id, *count))
                } else {
                    None
                }
            })
    }
}

impl<T> CellCounts<T> where T: Eq + Hash {
    /// Return a flattened map of barcode ids and their barcode and cell counts
    fn summary(&self, min_reads: usize) -> HashMap<&T, (usize, usize)> {
        let mut result = HashMap::new();
        self.0.values()
            .flat_map(|counter| counter.filter_hits(min_reads))
            .for_each(|(id, count)| {
                let c = result.entry(id).or_insert((0usize, 0usize));
                c.0 += count;
                c.1 += 1;
            });

        result
    }
}

impl<'a> Summary<'a> {
    pub fn new(barcodes: &'a Barcodes, counts: &'a Counts) -> Summary<'a> {
        Summary { counts, barcodes }
    }

   pub fn print_matches(
        &self,
        min_reads: usize,
        min_cells: usize,
        reads_per_cell: Option<usize>,
        tty: bool,
    ) {

        let table = self.gen_table(min_reads, min_cells, reads_per_cell);

        if tty {
            print!("{}", termion::cursor::Goto(1, 1));
        }
        print!("{}", table.display().unwrap());
        let cl: &str = termion::clear::AfterCursor.as_ref();

        println!(
            "{cl}\nIgnored: {}{cl}\nNo barcode hit: {}{cl}\nMultiple barcode hits: {}{cl}\nCellcodes not whitelisted: {}{cl}",
            self.counts.ignored, self.counts.nohit, self.counts.multiple, self.counts.not_whitelisted
        );
    }

    pub fn gen_table(
        &self,
        min_reads: usize,
        min_cells: usize,
        reads_per_cell: Option<usize>,
    ) -> TableStruct {
        let mut hits: Vec<_> = self
            .counts.cells
            .summary(min_reads)
            .into_iter()
            .map(|(pos, (count, cells))| (pos, count, cells))
            .collect();

        hits.sort_by_key(|e| e.1);

        let mut tabledata = Vec::new();
        for (pos, count, cells) in hits.into_iter().rev() {
            let record = &self.barcodes.records[*pos];

            let col = if passes(count, cells, min_reads, min_cells, reads_per_cell) {
                Some(Color::Green)
            } else {
                Some(Color::Red)
            };

            tabledata.push(vec![
                record.get(1).unwrap().cell().foreground_color(col),
                record.get(4).unwrap().cell().foreground_color(col),
                count.cell().justify(Justify::Right),
                cells.cell().justify(Justify::Right),
                (count / cells).cell().justify(Justify::Right),
            ]);
        }

        tabledata
            .table()
            .title(vec![
                "name".cell(),
                "barcode".cell(),
                format!("count (>{})", min_reads).cell(),
                format!("cells (>{})", min_cells).cell(),
                format!("reads/cell{}", if let Some(rpc) = reads_per_cell { format!(" (>{})", rpc)} else { "".to_owned() }).cell(),
            ])
            .border(Border::builder().build())
            .separator(Separator::builder().row(None).column(None).build())
    }


    pub fn print_unknown(&self, min_reads: usize) {
        let mut hits: Vec<_> = self.counts.unknown.summary(min_reads)
            .into_iter()
            .map(|(barcode, (count, cells))| (barcode, count, cells))
            .collect();
        hits.sort_by_key(|e| e.1);

        let mut tabledata = Vec::new();
        for (barcode, count, cells) in hits.iter().rev().take(20) {
            tabledata.push(vec![
                String::from_utf8_lossy(barcode.as_slice()).cell(),
                count.cell().justify(Justify::Right),
                cells.cell().justify(Justify::Right),
                (count / cells).cell().justify(Justify::Right),
            ]);
        }

        let table = tabledata
            .table()
            .title(vec![
                "barcode".cell(),
                format!("count (>{}/c)", min_reads).cell(),
                "cells".cell(),
                "reads/cell".cell(),
            ])
            .border(Border::builder().build())
            .separator(Separator::builder().row(None).column(None).build());

        println!("\nUnknown barcode summary ({} total):\n{}", hits.len(), table.display().unwrap());

    }

    pub fn write_csv<W: Write>(
        &self,
        w: W,
        min_reads: usize,
        min_cells: usize,
        reads_per_cell: Option<usize>,
    ) -> Result<()> {
        let result = self.counts.cells.summary(min_reads);

        self.barcodes.write_csv(
            w,
            result
                .into_iter()
                .filter(|&(_pos, (count, cells))| {
                    passes(count, cells, min_reads, min_cells, reads_per_cell)
                })
                .map(|(pos, _)| *pos),
        )
    }
}

/// Helper function for testing if the thresholds are met
fn passes(
    count: usize,
    cells: usize,
    min_reads: usize,
    min_cells: usize,
    reads_per_cell: Option<usize>,
) -> bool {
    count > min_reads && (cells >= min_cells || reads_per_cell.map_or(true, |r| count / cells > r))
}
