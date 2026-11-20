mod draw;

use dash_reader::odometer_detector::{read_odometer, OdometerReading};
use image::{DynamicImage, RgbImage};

fn detect(rgb: &RgbImage) -> Option<OdometerReading> {
    let luma = DynamicImage::ImageRgb8(rgb.clone()).to_luma8();
    let dim = (luma.width(), luma.height());
    read_odometer(&luma, Some(rgb), dim)
}

#[test]
fn detects_synthetic_odometer_with_red_tenths() {
    let rgb = draw::draw_dashboard(&[0, 4, 2, 1, 3, 7], true, false);
    let r = detect(&rgb).expect("odometer should be detected");
    assert_eq!(r.digits, vec![0, 4, 2, 1, 3, 7]);
    assert!((r.km - 4213.7).abs() < 1e-9);
    assert!(r.red_tenths);
    assert!(r.confidence > 0.5);
}

#[test]
fn detects_inverted_odometer() {
    let rgb = draw::draw_dashboard(&[9, 9, 9, 9, 9, 9], false, true);
    let r = detect(&rgb).expect("odometer should be detected");
    assert_eq!(r.digits, vec![9; 6]);
    assert!((r.km - 99999.9).abs() < 1e-9);
    assert!(!r.red_tenths);
}

#[test]
fn detects_five_digit_odometer() {
    let rgb = draw::draw_dashboard(&[1, 2, 3, 4, 5], false, false);
    let r = detect(&rgb).expect("odometer should be detected");
    assert_eq!(r.digits, vec![1, 2, 3, 4, 5]);
    assert!((r.km - 1234.5).abs() < 1e-9);
}

#[test]
fn rejects_image_without_odometer() {
    let rgb = draw::draw_dashboard(&[], false, false);
    assert!(detect(&rgb).is_none());
}
