use std::env;
use std::process;
use std::time::Instant;

use dash_reader::odometer_detector;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <image-file>", args[0]);
        process::exit(1);
    }
    read_image(&args[1]);
}

fn read_image(filename: &str) {
    let start = Instant::now();
    let img = match image::open(filename) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Could not open image {}: {}", filename, e);
            process::exit(1);
        }
    };
    println!("Image opened in {:?}", start.elapsed());

    let rgb = img.to_rgb8();
    let luma = img.to_luma8();
    let dim = (luma.width(), luma.height());
    match odometer_detector::read_odometer_any(&luma, Some(&rgb), dim) {
        Some(reading) => println!("{}", reading),
        None => eprintln!("No odometer reading found in {}", filename),
    }
    println!("Image processed in {:?}", start.elapsed());
}
