use std::fs::File;
use std::io;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

mod barcodes;
mod counts;
mod reader;
mod whitelist;

use barcodes::*;
use counts::*;
use whitelist::Whitelist;

pub const CCLENGTH: usize = 16;
pub const BCLENGTH: usize = 15;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Provide the totalseq style csv file with the antibody barcodes
    #[arg(long)]
    csv: PathBuf,

    /// Provide the 10X barcodes whitelist file
    #[arg(long)]
    whitelist: Option<PathBuf>,

    /// The feature barcode read 1 FastQ file containing the cell codes
    r1: PathBuf,

    /// The feature barcode read 2 FastQ file containing the barcodes
    r2: PathBuf,

    /// Minimum barcode reads per cellcode.
    /// Only count the barcodes that are found more than <B> times for a cell code
    #[arg(long, short = 'b', default_value_t = 5)]
    min_reads: usize,

    /// Minimum number of cells having an accepted barcode.
    /// Only output the barcodes that are found in more than <C> cells
    #[arg(long, short = 'c', default_value_t = 5)]
    min_cells: usize,

    /// Reads per cell
    /// Only output the barcodes that on average have more than <R> reads per cell
    #[arg(long, short = 'r')]
    reads_per_cell: Option<usize>,

    /// Out CSV for 10X cellranger
    #[arg(long, short = 'o')]
    out: Option<PathBuf>,
}

fn main() -> Result<()> {
    let config = Config::parse();

    let tty = termion::is_tty(&io::stdout());
    if tty {
        println!("{}", termion::clear::All);
    }

    // open the FastQ pair
    let mut reader = reader::Reader::from_paths(&config.r1, &config.r2)?;

    // initialize the count structs
    let barcodes = Barcodes::from_csv(&config.csv)?;
    let mut counts = Counts::default();

    // optionally read the whitelist
    let ws = config
        .whitelist
        .map(|ws| Whitelist::from_path(ws))
        .transpose()?;

    let mut count = 0;

    let mut cc = [0u8; 16];
    let mut bc = [0u8; 15];

    while let Some(result) = reader.read_code(&mut cc, &mut bc) {
        let _result = result?;

        //check whitelisted
        if let Some(l) = &ws {
            if !l.contains(cc.as_slice()) {
                counts.not_whitelisted();
                continue;
            }
        }

        let result = barcodes.find(&bc);
        match result {
            MatchResult::Unique(pos) => counts.count_barcode(&cc, pos),
            MatchResult::Dist(pos, _dist) => counts.count_barcode(&cc, pos),
            MatchResult::NoHit => counts.nohit(),
            MatchResult::Multiple => counts.multiple(),
        }

        count += 1;

        //update live stats if interactive tty
        if tty {
            if count % 500_000 == 0 {
                let summary = Summary::new(&barcodes, &counts);
                summary.print_matches(
                    config.min_reads,
                    config.min_cells,
                    config.reads_per_cell,
                    tty,
                );
            }
        }
    }

    let summary = Summary::new(&barcodes, &counts);
    summary.print_matches(
        config.min_reads,
        config.min_cells,
        config.reads_per_cell,
        tty,
    );
    if let Some(out) = config.out {
        let f = File::create(out)?;
        summary.write_csv(f, config.min_cells, config.min_reads, config.reads_per_cell)?;
    }

    Ok(())
}
