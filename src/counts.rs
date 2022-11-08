use std::io::Write;

use ahash::AHashMap as HashMap;
use anyhow::Result;
use cli_table::{
    format::{Border, Justify, Separator},
    Cell, Color, Style, Table, TableStruct,
};

use crate::barcodes::Barcodes;
use crate::CCLENGTH;

/// Count the barcode (usize references) per cellcode
#[derive(Default)]
pub struct Counts {
    cells: HashMap<Vec<u8>, HashMap<usize, usize>>,
    multiple: usize,
    nohit: usize,
    not_whitelisted: usize,
}

pub struct Summary<'a> {
    barcodes: &'a Barcodes,
    counts: &'a Counts,
}

impl Counts {
    pub fn count_barcode(&mut self, cellcode: &[u8; CCLENGTH], pos: usize) {
        if let Some(cell) = self.cells.get_mut(cellcode.as_slice()) {
            let count = cell.entry(pos).or_insert(0);
            *count += 1;
        } else {
            let cell = [(pos, 1)].into_iter().collect();
            self.cells.insert(cellcode.as_slice().to_vec(), cell);
        }
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

impl<'a> Summary<'a> {
    pub fn new(barcodes: &'a Barcodes, counts: &'a Counts) -> Summary<'a> {
        Summary { counts, barcodes }
    }

    pub fn summarize(&self, min_reads: usize) -> HashMap<usize, (usize, usize)> {
        let mut result = HashMap::new();
        self.counts
            .cells
            .values()
            .flat_map(|v| {
                v.iter().filter_map(|(pos, count)| {
                    if *count > min_reads {
                        Some((pos, count))
                    } else {
                        None
                    }
                })
            })
            .for_each(|(&pos, count)| {
                let c = result.entry(pos).or_insert((0usize, 0usize));
                c.0 += count;
                c.1 += 1;
            });

        result
    }

    pub fn print_matches(
        &self,
        min_cells: usize,
        min_reads: usize,
        reads_per_cell: Option<usize>,
        tty: bool,
    ) {
        let mut hits: Vec<_> = self
            .summarize(min_reads)
            .into_iter()
            .map(|(pos, (count, cells))| (pos, count, cells))
            .collect();

        hits.sort_by_key(|e| e.1);

        let table = self.gen_table(min_cells, min_reads, reads_per_cell);

        if tty {
            print!("{}", termion::cursor::Goto(1, 1));
        }
        print!("{}", table.display().unwrap());
        let cl: &str = termion::clear::AfterCursor.as_ref();

        println!(
            "{cl}\nNo barcode hit: {}{cl}\nMultiple barcode hits: {}{cl}\nCellcodes not whitelisted: {}{cl}",
            self.counts.nohit, self.counts.multiple, self.counts.not_whitelisted
        );
    }

    pub fn gen_table(
        &self,
        min_cells: usize,
        min_reads: usize,
        reads_per_cell: Option<usize>,
    ) -> TableStruct {
        let mut hits: Vec<_> = self
            .summarize(min_reads)
            .into_iter()
            .map(|(pos, (count, cells))| (pos, count, cells))
            .collect();

        hits.sort_by_key(|e| e.1);

        let mut tabledata = Vec::new();
        for (pos, count, cells) in hits.into_iter().rev() {
            let record = &self.barcodes.records[pos];

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

    pub fn write_csv<W: Write>(
        &self,
        w: W,
        min_cells: usize,
        min_reads: usize,
        reads_per_cell: Option<usize>,
    ) -> Result<()> {
        let result = self.summarize(min_reads);

        self.barcodes.write_csv(
            w,
            result
                .into_iter()
                .filter(|&(_pos, (count, cells))| {
                    passes(count, cells, min_cells, min_reads, reads_per_cell)
                })
                .map(|(pos, _)| pos),
        )
    }
}

fn passes(
    count: usize,
    cells: usize,
    min_cells: usize,
    min_reads: usize,
    reads_per_cell: Option<usize>,
) -> bool {
    cells >= min_cells && count > min_reads && reads_per_cell.map_or(true, |r| count / cells > r)
}
