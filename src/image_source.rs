use crate::odometer_detector::{ColorValue, PixelValue};

impl PixelValue for image::GrayImage {
    fn get_row_values(&self, y: u32, row: &mut [u8]) {
        let width = self.width() as usize;
        row.copy_from_slice(&self.as_raw()[y as usize * width..][..width]);
    }

    fn get_col_values(&self, x: u32, col: &mut [u8]) {
        let width = self.width() as usize;
        let raw = self.as_raw();
        for (i, c) in col.iter_mut().enumerate() {
            *c = raw[i * width + x as usize];
        }
    }
}

impl ColorValue for image::RgbImage {
    fn get_pixel_rgb(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let p = self.get_pixel(x, y);
        (p[0], p[1], p[2])
    }
}
