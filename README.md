# Featureseek

## Introduction
`featureseek` is a small command line tool used to scan the FastQ files for the
Feature Barcode reads of a 10X experiment.

It uses a Biolegend TotalSeq reference list to quickly verify the feature
barcodes before running the `cellranger` pipeline. It can write a cellranger
compatible CSV file.

## Usage
```
Usage: featureseek [OPTIONS] --csv <CSV> <R1> <R2>

Arguments:
  <R1>  The feature barcode read 1 FastQ file containing the cell codes
  <R2>  The feature barcode read 2 FastQ file containing the barcodes

Options:
      --csv <CSV>           Provide the totalseq csv file with the antibody barcodes
      --whitelist <FILE>    Provide the 10X barcodes whitelist file
  -b, --min-reads <B>       Minimum barcode reads per cellcode. Only count the barcodes that are found more than <B> times for a cell code [default: 5]
  -c, --min-cells <C>       Minimum number of cells having an accepted barcode. Only output the barcodes that are found in more than <C> cells [default: 5]
  -r, --reads-per-cell <R>  Reads per cell. Only output the barcodes that on average have more than <R> reads per cell
  -o, --out <OUT>           Out CSV for 10X cellranger
  -x, --ignore <BC,BC,...>  Barcode ignore list [default: GGGGGGGGGGGGGGG,CCTAATGGTCCAGAC]
  -u, --unknown             Count unknown. Count the barcodes not matching to the reference as summarize at end
  -a, --approximate         Approximate matching. Count the barcodes allowing a levenshtein distance up to 2 to the reference
  -h, --help                Print help information
  -V, --version             Print version information
```

`featureseek` requires both the read 1 and read 2 FastQ files. While running a
table will be updated with summary results. All barcodes passing the
`min-reads` are displayed. The barcodes passing all provided thresholds are
listed in green. When `--out` is provided the green barcodes will be written to
a `Cell Ranger` compatible CSV file.

## Method
`featureseek` counts the barcode occurrences per cellcode. When the 10X
cellcode whitelist is provided, only the whitelisted cellcodes are used. In
order to minimize noise only barcodes that are found on cells more than the
`--min_reads` option are analysed. To further select the true features two
additional options are available.  `--min-cells C` requires the number of cell
with that barcode the be at least `C`. `--reads-per-cell R` requires the average
number of reads per (positive) cell to be at least `R`.

## Required data
The TotalSeq CSV file can be found at the [BioLegend website](https://www.biolegend.com/en-us/totalseq/barcode-lookup). Use the
Cell Ranger export function to retrieve the CSV file. The antibody and hashing
tables can be combined into a single CSV.

10X barcode whitelists can be found in the `Cell Ranger` installation
directory:
```
cellranger-7.0.1/lib/python/cellranger/barcodes/*.txt
```

## Finally
This is a QC tool, not a quantification tool. No cellcodes are selected except
the optional whitelist. Expect higher numbers than from the final `Cell Ranger` pipeline.


