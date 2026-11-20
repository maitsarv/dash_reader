#![allow(dead_code)]

use dash_reader::digit_translate::*;
use image::{Rgb, RgbImage};

fn fill_rect(img: &mut RgbImage, x0: u32, y0: u32, x1: u32, y1: u32, c: Rgb<u8>) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            img.put_pixel(x, y, c);
        }
    }
}

pub fn draw_digit(img: &mut RgbImage, digit: u8, x: u32, y: u32, w: u32, h: u32, color: Rgb<u8>) {
    let t = (w / 5).max(4);
    let seg = DIGIT_SEGMENTS[digit as usize].1;
    let hx0 = x + 2;
    let hx1 = x + w - 3;
    if seg & SEG_TOP != 0 {
        fill_rect(img, hx0, y + 2, hx1, y + 2 + t - 1, color);
    }
    if seg & SEG_MIDDLE != 0 {
        fill_rect(img, hx0, y + h / 2 - t / 2, hx1, y + h / 2 + t / 2 - 1, color);
    }
    if seg & SEG_BOTTOM != 0 {
        fill_rect(img, hx0, y + h - 2 - t, hx1, y + h - 3, color);
    }
    let vh = h / 2 - 4;
    let vy0u = y + 2 + t / 2;
    let vy1u = y + 2 + t / 2 + vh - 1;
    let vy0l = y + h - 2 - t / 2 - vh;
    let vy1l = y + h - 2 - t / 2 - 1;
    if seg & SEG_UPPER_LEFT != 0 {
        fill_rect(img, x + 2, vy0u, x + 2 + t - 1, vy1u, color);
    }
    if seg & SEG_UPPER_RIGHT != 0 {
        fill_rect(img, x + w - 3 - t, vy0u, x + w - 3, vy1u, color);
    }
    if seg & SEG_LOWER_LEFT != 0 {
        fill_rect(img, x + 2, vy0l, x + 2 + t - 1, vy1l, color);
    }
    if seg & SEG_LOWER_RIGHT != 0 {
        fill_rect(img, x + w - 3 - t, vy0l, x + w - 3, vy1l, color);
    }
}

pub fn draw_dashboard(value: &[u8], red_last: bool, invert: bool) -> RgbImage {
    let (w, h) = (900u32, 360u32);
    let mut img = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let g = 40 + (y * 40 / h) as u8;
            let noise = ((x.wrapping_mul(7) + y.wrapping_mul(13)) % 7) as u8;
            img.put_pixel(x, y, Rgb([g + noise, g + noise, g + 5 + noise]));
        }
    }
    for y in 0..h {
        for x in 0..w {
            let dx = x as i32 - 820;
            let dy = y as i32 - 40;
            if dx * dx + dy * dy < 50 * 50 {
                let p = img.get_pixel(x, y);
                img.put_pixel(
                    x,
                    y,
                    Rgb([
                        p[0].saturating_add(60),
                        p[1].saturating_add(60),
                        p[2].saturating_add(60),
                    ]),
                );
            }
        }
    }
    let (wx0, wy0, wx1, wy1) = (120u32, 100u32, 780u32, 260u32);
    let (wbg, wfg) = if invert {
        (Rgb([225u8, 225, 220]), Rgb([25u8, 25, 30]))
    } else {
        (Rgb([12u8, 12, 16]), Rgb([228u8, 228, 222]))
    };
    fill_rect(&mut img, wx0, wy0, wx1, wy1, wbg);
    let n = value.len() as u32;
    if n > 0 {
        let dw = 70u32;
        let gap = 20u32;
        let dh = 120u32;
        let total = n * dw + (n - 1) * gap;
        let mut x = wx0 + (wx1 - wx0 - total) / 2;
        let y = wy0 + (wy1 - wy0 - dh) / 2;
        for (i, &d) in value.iter().enumerate() {
            let color = if red_last && i == value.len() - 1 {
                Rgb([215u8, 45, 40])
            } else {
                wfg
            };
            draw_digit(&mut img, d, x, y, dw, dh, color);
            x += dw + gap;
        }
    }
    img
}
