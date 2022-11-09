use std::fs::File;
use std::io;
use std::path::PathBuf;

use ahash::AHashSet as HashSet;
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

pub type CellCode = [u8; CCLENGTH];
pub type Barcode = [u8; BCLENGTH];
pub type BarcodeRef = usize;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Provide the TotalSeq style csv file with the antibody barcodes
    #[arg(long)]
    csv: PathBuf,

    /// Provide the 10X barcodes whitelist file
    #[arg(long, value_name = "FILE")]
    whitelist: Option<PathBuf>,

    /// The feature barcode read 1 FastQ file containing the cell codes.
    r1: PathBuf,

    /// The feature barcode read 2 FastQ file containing the barcodes.
    r2: PathBuf,

    /// Minimum barcode reads per cellcode.
    /// Only count the barcodes that are found more than <B> times for a cell code.
    #[arg(long, short = 'b', value_name = "B", default_value_t = 5)]
    min_reads: usize,

    /// Minimum number of cells having an accepted barcode.
    /// Only output the barcodes that are found in more than <C> cells.
    #[arg(long, short = 'c', value_name = "C", default_value_t = 5)]
    min_cells: usize,

    /// Reads per cell.
    /// Only output the barcodes that on average have more than <R> reads per cell.
    #[arg(long, short = 'r', value_name = "R")]
    reads_per_cell: Option<usize>,

    /// Out hashtag CSV for 10X cellranger pipeline.
    #[arg(long, short = 'o')]
    out: Option<PathBuf>,

    /// Barcode ignore list.
    #[arg(long, short = 'x', value_name = "BC,BC,...", value_parser = parse_ignores, default_value = "GGGGGGGGGGGGGGG,CCTAATGGTCCAGAC")]
    ignore: HashSet<Vec<u8>>,

    /// Count unknown.
    /// Count the barcodes not matching to the reference and summarize at end.
    #[arg(long, short = 'u')]
    unknown: bool,

    /// Approximate matching.
    /// Count the barcodes allowing a levenshtein distance up to 2 to the reference.
    #[arg(long, short = 'a')]
    approximate: bool,
}

fn parse_ignores(s: &str) -> Result<HashSet<Vec<u8>>> {
    Ok(s.split(',').map(|p| p.as_bytes().to_vec()).collect())
}

fn main() -> Result<()> {
    let config = Config::parse();
    let has_ignore = !config.ignore.is_empty();

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
        .map(Whitelist::from_path)
        .transpose()?;

    let mut count = 0;

    let mut cc = [0u8; CCLENGTH];
    let mut bc = [0u8; BCLENGTH];

    while let Some(result) = reader.read_code(&mut cc, &mut bc) {
        result?;
        count += 1;

        //check whitelisted
        if let Some(l) = &ws {
            if !l.contains(cc.as_slice()) {
                counts.not_whitelisted();
                continue;
            }
        }

        if has_ignore && config.ignore.contains(bc.as_slice()) {
            counts.ignored();
            continue;
        }

        let result = barcodes.find(&bc, config.approximate);
        match result {
            MatchResult::Unique(pos) => counts.count_barcode(cc, pos),
            MatchResult::Dist(pos, _dist) => counts.count_barcode(cc, pos),
            MatchResult::NoHit => {
                if config.unknown {
                    counts.count_unknown(cc, bc);
                }
                counts.nohit();
            }
            MatchResult::Multiple => counts.multiple(),
        }

        //update live stats if interactive tty
        if tty && count % 500_000 == 0 {
            let summary = Summary::new(&barcodes, &counts);
            summary.print_matches(
                config.min_reads,
                config.min_cells,
                config.reads_per_cell,
                tty,
                );
        }
    }

    let summary = Summary::new(&barcodes, &counts);
    summary.print_matches(
        config.min_reads,
        config.min_cells,
        config.reads_per_cell,
        tty,
    );
    println!("Examined {count} reads");

    if config.unknown {
        summary.print_unknown(config.min_reads);
    }

    if let Some(out) = config.out {
        let f = File::create(out)?;
        summary.write_csv(f, config.min_cells, config.min_reads, config.reads_per_cell)?;
    }

    Ok(())
}
