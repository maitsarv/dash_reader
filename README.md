# Dash reader
Personal project for learning Rust language, inspired by [rust_barcode_reader](https://github.com/maitsarv/rust_barcode_reader).

# About the project
* Reads the odometer value from a car dashboard photo.
* Detects 7-segment style odometer digits (mechanical rollers and LCD clusters).
* Image processing part is very lightweight to make the detection fast: row scanning with an adaptive step, local threshold crossing detection and 7-segment template matching. No ML.
* The last digit is treated as tenths of a kilometre; on mechanical odometers that digit is usually red, which is detected from the RGB image.
* Multi-row voting plus template-match margin reject ambiguous cells (similar role to the barcode checksum).
* Does not go over every pixel, the row step is calculated based on the image height.

# Usage
```
cargo run -- <image-file>
```

# Implementation
The current implementation reads image files.
Pipeline: find the high-contrast band of the instrument cluster -> segment digit cells by column profiles -> classify each cell against 7-segment templates with voting across rows -> assemble the km value.

Synthetic test images are generated with:
```
cargo run --example make_sample
```
which writes `test/img/sample_dashboard.png`.
