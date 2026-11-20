use std::cmp::{max, min};

use crate::color_line_helpers::{find_crossings_from_average, ColorLine};
use crate::digit_translate::{
    match_digit, SEG_BOTTOM, SEG_LOWER_LEFT, SEG_LOWER_RIGHT, SEG_MIDDLE, SEG_TOP,
    SEG_UPPER_LEFT, SEG_UPPER_RIGHT,
};

pub trait PixelValue {
    fn get_row_values(&self, y: u32, row: &mut [u8]);
    fn get_col_values(&self, x: u32, col: &mut [u8]);
}

pub trait ColorValue {
    fn get_pixel_rgb(&self, x: u32, y: u32) -> (u8, u8, u8);
}

#[derive(Debug)]
pub struct OdometerReading {
    pub digits: Vec<u8>,
    pub km: f64,
    pub red_tenths: bool,
    pub bbox: (u32, u32, u32, u32),
    pub confidence: f32,
}

impl std::fmt::Display for OdometerReading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        let n = self.digits.len();
        for (i, d) in self.digits.iter().enumerate() {
            if i + 1 == n && n > 1 {
                s.push('.');
            }
            s.push((b'0' + d) as char);
        }
        write!(f, "{} km (confidence {:.0}%)", s, self.confidence * 100.0)
    }
}

pub fn read_odometer(
    img: &dyn PixelValue,
    color: Option<&dyn ColorValue>,
    dim: (u32, u32),
) -> Option<OdometerReading> {
    let (y0, y1, band_range, cross_hist) = find_band(img, dim)?;
    let (x0, x1) = find_x_range(&cross_hist)?;
    let ink = column_ink(img, dim, y0, y1);
    let cells = segment_cells(&ink, x0, x1);
    if cells.len() < 4 || cells.len() > 8 {
        return None;
    }
    let mut digits = Vec::with_capacity(cells.len());
    let mut conf_sum = 0.0f32;
    for cell in &cells {
        let (d, c) = classify_cell(img, dim, *cell, y0, y1, band_range)?;
        digits.push(d);
        conf_sum += c;
    }
    let confidence = conf_sum / cells.len() as f32;
    let mut value = 0u64;
    for &d in &digits {
        value = value * 10 + d as u64;
    }
    let red_tenths = match color {
        Some(c) => is_red_digit(c, *cells.last().unwrap(), y0, y1),
        None => false,
    };
    Some(OdometerReading {
        digits,
        km: value as f64 / 10.0,
        red_tenths,
        bbox: (x0 as u32, y0, x1 as u32, y1),
        confidence,
    })
}

fn calculate_row_step(h: u32) -> u32 {
    max(2, ((h as f64).log10() * 2.0) as u32)
}

fn build_line(img: &dyn PixelValue, y: u32, row_buf: &mut [u8], slice_size: usize) -> ColorLine {
    let len = row_buf.len();
    let mut line = ColorLine {
        avg: 0,
        min: 255,
        max: 0,
        deg: 0,
        pos: y,
        values: vec![0u8; len],
        avg_loc: Vec::with_capacity(len / slice_size + 1),
        min_loc: Vec::with_capacity(len / slice_size + 1),
        max_loc: Vec::with_capacity(len / slice_size + 1),
        slice_size,
    };
    img.get_row_values(y, row_buf);
    line.values.copy_from_slice(row_buf);
    let mut line_sum = 0usize;
    let mut slc = 0usize;
    let mut sv = (255u8, 0u8, 0usize);
    for x in 0..len {
        let px = line.values[x];
        sv.2 += px as usize;
        line_sum += px as usize;
        if px < sv.0 {
            sv.0 = px;
            if px < line.min {
                line.min = px;
            }
        }
        if px > sv.1 {
            sv.1 = px;
            if px > line.max {
                line.max = px;
            }
        }
        slc += 1;
        if slc >= slice_size {
            line.min_loc.push(sv.0);
            line.max_loc.push(sv.1);
            line.avg_loc.push((sv.2 / slc) as u8);
            sv = (255, 0, 0);
            slc = 0;
        }
    }
    if slc > 0 {
        line.min_loc.push(sv.0);
        line.max_loc.push(sv.1);
        line.avg_loc.push((sv.2 / slc) as u8);
    }
    line.avg = (line_sum / len.max(1)) as u8;
    line
}

pub fn find_band(img: &dyn PixelValue, dim: (u32, u32)) -> Option<(u32, u32, f32, Vec<f32>)> {
    let (w, h) = (dim.0 as usize, dim.1 as usize);
    if w < 40 || h < 20 {
        return None;
    }
    let step = calculate_row_step(dim.1) as usize;
    let slice_size = max(20, max(w, h) / 40);
    let big_image = slice_size > 40;
    let mut row_buf = vec![0u8; w];
    let mut scores: Vec<(u32, f32, usize, u8)> = Vec::new();
    let mut all_cross: Vec<Vec<usize>> = Vec::new();
    let mut y = 0usize;
    while y < h {
        let line = build_line(img, y as u32, &mut row_buf, slice_size);
        let crossings = find_crossings_from_average(&line, big_image);
        let contrast = line.max - line.min;
        let n = crossings.1.len();
        let score = n as f32 * ((contrast as f32) / 96.0).min(1.0);
        scores.push((y as u32, score, n, contrast));
        all_cross.push(crossings.1);
        y += step;
    }
    let max_score = scores.iter().map(|s| s.1).fold(0.0f32, f32::max);
    if max_score < 8.0 {
        return None;
    }
    let thresh = max_score * 0.3;
    let mut best = (0usize, 0usize, 0.0f32);
    let mut cur_start = 0usize;
    let mut cur_sum = 0.0f32;
    let mut gap = 0usize;
    for (i, &(_, score, n, _)) in scores.iter().enumerate() {
        if score >= thresh && n >= 8 {
            if cur_sum <= 0.0 {
                cur_start = i;
            }
            cur_sum += score;
            gap = 0;
        } else {
            gap += 1;
            if gap > 3 {
                if cur_sum > best.2 {
                    best = (cur_start, i - gap + 1, cur_sum);
                }
                cur_sum = 0.0;
            }
        }
    }
    if cur_sum > best.2 {
        best = (cur_start, scores.len(), cur_sum);
    }
    if best.2 <= 0.0 || best.1 <= best.0 {
        return None;
    }
    let y0 = scores[best.0].0.saturating_sub(step as u32);
    let y1 = min(scores[best.1 - 1].0 + step as u32, dim.1);
    if y1 <= y0 + 8 || y1 - y0 > dim.1 / 2 {
        return None;
    }
    let band_range = scores[best.0..best.1].iter().map(|s| s.3).max().unwrap_or(0);
    let mut hist = vec![0f32; w];
    for i in best.0..best.1 {
        for &cx in &all_cross[i] {
            if cx < w {
                hist[cx] += 1.0;
            }
        }
    }
    let mut sm = vec![0f32; w];
    for x in 0..w {
        let mut s = 0.0f32;
        let mut c = 0.0f32;
        for d in -2i32..=2 {
            let xx = x as i32 + d;
            if xx >= 0 && (xx as usize) < w {
                s += hist[xx as usize];
                c += 1.0;
            }
        }
        sm[x] = s / c;
    }
    Some((y0, y1, band_range as f32, sm))
}

pub fn find_x_range(cross: &[f32]) -> Option<(usize, usize)> {
    let total: f32 = cross.iter().sum();
    if total < 40.0 {
        return None;
    }
    let mut acc = 0.0f32;
    let mut x0 = None;
    for (i, &v) in cross.iter().enumerate() {
        acc += v;
        if x0.is_none() && acc >= total * 0.05 {
            x0 = Some(i);
        }
    }
    let mut acc = 0.0f32;
    let mut x1 = None;
    for (i, &v) in cross.iter().enumerate().rev() {
        acc += v;
        if x1.is_none() && acc >= total * 0.05 {
            x1 = Some(i);
        }
    }
    let (x0, x1) = (x0?, x1?);
    if x1 < x0 + 30 {
        return None;
    }
    Some((x0, x1))
}

pub fn column_ink(img: &dyn PixelValue, dim: (u32, u32), y0: u32, y1: u32) -> Vec<f32> {
    let (w, h) = (dim.0 as usize, dim.1 as usize);
    let mut col_buf = vec![0u8; h];
    let mut col_med = vec![0u8; w];
    let mut col_range = vec![0u8; w];
    let mut band_min = 255u8;
    let mut band_max = 0u8;
    for x in 0..w {
        img.get_col_values(x as u32, &mut col_buf);
        let seg = &col_buf[y0 as usize..y1 as usize];
        let mut sorted = seg.to_vec();
        sorted.sort_unstable();
        col_med[x] = sorted[sorted.len() / 2];
        let mn = sorted[0];
        let mx = sorted[sorted.len() - 1];
        col_range[x] = mx - mn;
        if mn < band_min {
            band_min = mn;
        }
        if mx > band_max {
            band_max = mx;
        }
    }
    let band_range = (band_max - band_min) as f32;
    let thresh = band_range * 0.3;
    let win = 15usize;
    let mut ink = vec![0f32; w];
    for x in 0..w {
        let l = x.saturating_sub(win);
        let r = min(x + win + 1, w);
        let mut neighborhood: Vec<u8> = col_med[l..r].to_vec();
        neighborhood.sort_unstable();
        let bg_local = neighborhood[neighborhood.len() / 2] as f32;
        let v = if col_range[x] >= 24 {
            col_range[x] as f32
        } else {
            let d = (col_med[x] as f32 - bg_local).abs();
            if d > thresh {
                d
            } else {
                0.0
            }
        };
        ink[x] = v;
    }
    let mut sm = vec![0f32; w];
    for x in 0..w {
        let mut s = 0.0f32;
        let mut c = 0.0f32;
        for d in -2i32..=2 {
            let xx = x as i32 + d;
            if xx >= 0 && (xx as usize) < w {
                s += ink[xx as usize];
                c += 1.0;
            }
        }
        sm[x] = s / c;
    }
    sm
}

pub fn segment_cells(ink: &[f32], x0: usize, x1: usize) -> Vec<(u32, u32)> {
    let seg = &ink[x0..=x1];
    let max_ink = seg.iter().fold(0.0f32, |a, &b| a.max(b));
    if max_ink <= 0.0 {
        return Vec::new();
    }
    let t = max_ink * 0.12;
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    for (i, &v) in seg.iter().enumerate() {
        if v >= t {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            runs.push((s, i - 1));
        }
    }
    if let Some(s) = start {
        runs.push((s, seg.len() - 1));
    }
    if runs.is_empty() {
        return Vec::new();
    }
    let max_w = runs.iter().map(|r| r.1 - r.0 + 1).max().unwrap_or(1);
    let mut widths: Vec<usize> = runs
        .iter()
        .map(|r| r.1 - r.0 + 1)
        .filter(|&wd| wd >= max_w / 2)
        .collect();
    if widths.is_empty() {
        widths.push(max_w);
    }
    widths.sort_unstable();
    let pitch = widths[widths.len() / 2].max(4);
    let mut subs: Vec<(usize, usize)> = Vec::new();
    for (s, e) in runs {
        let wd = e - s + 1;
        if wd < 5 {
            continue;
        }
        let n = max(1, ((wd as f32) / (pitch as f32)).round() as usize);
        for k in 0..n {
            subs.push((s + (wd * k) / n, s + (wd * (k + 1)) / n - 1));
        }
    }
    let mut cells: Vec<(u32, u32)> = Vec::new();
    for i in 0..subs.len() {
        let a = if i == 0 {
            x0 + subs[0].0
        } else {
            x0 + (subs[i - 1].1 + subs[i].0 + 1) / 2
        };
        let b = if i + 1 == subs.len() {
            x0 + subs[i].1
        } else {
            x0 + (subs[i].1 + subs[i + 1].0) / 2
        };
        cells.push((a as u32, b as u32));
    }
    let half = (pitch / 2) as u32;
    if let Some(f) = cells.first_mut() {
        f.0 = f.0.saturating_sub(half);
    }
    if let Some(l) = cells.last_mut() {
        l.1 = min(l.1 + half, ink.len() as u32 - 1);
    }
    cells
}

pub fn classify_cell(
    img: &dyn PixelValue,
    dim: (u32, u32),
    cell: (u32, u32),
    y0: u32,
    y1: u32,
    band_range: f32,
) -> Option<(u8, f32)> {
    if band_range < 30.0 {
        return None;
    }
    let w = dim.0 as usize;
    let band_h = (y1 - y0 + 1) as usize;
    let mut band_px = vec![0u8; w * band_h];
    let mut row_buf = vec![0u8; w];
    for yy in y0..=y1 {
        img.get_row_values(yy, &mut row_buf);
        band_px[(yy - y0) as usize * w..][..w].copy_from_slice(&row_buf);
    }
    let cx0 = cell.0 as usize;
    let cx1 = cell.1 as usize;
    let cw = cx1 - cx0 + 1;
    let mut cell_min = 255u8;
    let mut cell_max = 0u8;
    for r in 0..band_h {
        for x in cx0..=cx1 {
            let v = band_px[r * w + x];
            if v < cell_min {
                cell_min = v;
            }
            if v > cell_max {
                cell_max = v;
            }
        }
    }
    let cell_range = (cell_max - cell_min) as f32;
    if cell_range < 20.0 {
        return None;
    }
    let delta = (cell_range * 0.3).max(8.0);
    let m = max(3, cw / 4);
    let l0 = cx0.saturating_sub(m);
    let r1 = min(cx1 + m, w - 1);
    let mut ring: Vec<u8> = Vec::with_capacity(band_h * (m + m));
    for r in 0..band_h {
        for x in l0..cx0 {
            ring.push(band_px[r * w + x]);
        }
        for x in cx1 + 1..=r1 {
            ring.push(band_px[r * w + x]);
        }
    }
    if ring.is_empty() {
        return None;
    }
    ring.sort_unstable();
    let bg = ring[ring.len() / 2] as f32;
    let offsets = [-3i32, -1, 1, 3];
    let mut votes: Vec<(u8, u32, u32)> = Vec::new();
    for &off in &offsets {
        if let Some(v) = classify_at_offset(&band_px, w, cell, band_h, bg, delta, off) {
            votes.push(v);
        }
    }
    if votes.len() < 3 {
        return None;
    }
    let mut counts = [0usize; 10];
    for v in &votes {
        counts[v.0 as usize] += 1;
    }
    let mut best_digit = 0usize;
    for d in 1..10 {
        if counts[d] > counts[best_digit] {
            best_digit = d;
        }
    }
    if counts[best_digit] < 3 {
        return None;
    }
    let confidence = counts[best_digit] as f32 / votes.len() as f32;
    Some((best_digit as u8, confidence))
}

fn classify_at_offset(
    band_px: &[u8],
    w: usize,
    cell: (u32, u32),
    band_h: usize,
    bg: f32,
    delta: f32,
    off: i32,
) -> Option<(u8, u32, u32)> {
    let cx0 = cell.0 as usize;
    let cx1 = cell.1 as usize;
    let cw = cx1 - cx0 + 1;
    let mut row_ink = vec![0f32; band_h];
    for r in 0..band_h {
        let s = &band_px[r * w + cx0..r * w + cx0 + cw];
        for &v in s {
            if (v as f32 - bg).abs() > delta {
                row_ink[r] += 1.0;
            }
        }
    }
    let max_ink = row_ink.iter().fold(0.0f32, |a, &b| a.max(b));
    if max_ink < 3.0 {
        return None;
    }
    let t = max_ink * 0.15;
    let mut dy0 = 0usize;
    let mut dy1 = band_h - 1;
    for (i, &v) in row_ink.iter().enumerate() {
        if v >= t {
            dy0 = i;
            break;
        }
    }
    for (i, &v) in row_ink.iter().enumerate().rev() {
        if v >= t {
            dy1 = i;
            break;
        }
    }
    dy0 = (dy0 as i32 + off).max(0) as usize;
    dy1 = (dy1 as i32 + off).min((band_h - 1) as i32) as usize;
    if dy1 < dy0 + 7 {
        return None;
    }
    let gh = (dy1 - dy0 + 1) as f32;
    let gw = cw as f32;
    if gh < 8.0 || gw < 6.0 {
        return None;
    }
    let mut col_ink = vec![0f32; cw];
    for x in 0..cw {
        let mut c = 0.0f32;
        for r in 0..band_h {
            let v = band_px[r * w + cx0 + x];
            if (v as f32 - bg).abs() > delta {
                c += 1.0;
            }
        }
        col_ink[x] = c;
    }
    let max_ci = col_ink.iter().fold(0.0f32, |a, &b| a.max(b));
    let t_ci = max_ci * 0.4;
    let cap = max(4, (cw as f32 * 0.4) as usize);
    let mut lz: Option<(usize, usize)> = None;
    let mut x = 0usize;
    while x < cw && col_ink[x] < t_ci {
        x += 1;
    }
    if x < cw {
        let mut e = x;
        while e + 1 < cw && e + 1 - x < cap && col_ink[e + 1] >= t_ci {
            e += 1;
        }
        lz = Some((cx0 + x, cx0 + e));
    }
    let mut rz: Option<(usize, usize)> = None;
    if cw > 0 {
        let mut x = cw - 1;
        loop {
            if col_ink[x] >= t_ci {
                let mut s = x;
                while s > 0 && x - s + 1 < cap && col_ink[s - 1] >= t_ci {
                    s -= 1;
                }
                rz = Some((cx0 + s, cx0 + x));
                break;
            }
            if x == 0 {
                break;
            }
            x -= 1;
        }
    }
    let mid_x = cx0 + cw / 2;
    let (lz, rz): (Option<(usize, usize)>, Option<(usize, usize)>) = match (lz, rz) {
        (Some(l), Some(r)) if l == r => {
            if (l.0 + l.1) / 2 < mid_x {
                (Some(l), None)
            } else {
                (None, Some(r))
            }
        }
        (l, r) => (l, r),
    };
    let (hx0, hx1) = match (&lz, &rz) {
        (Some(l), Some(r)) => (l.1 + 1, r.0.saturating_sub(1)),
        (Some(l), None) => (l.1 + 1, cx1),
        (None, Some(r)) => (cx0, r.0.saturating_sub(1)),
        (None, None) => (cx0 + (gw * 0.20) as usize, cx1 - (gw * 0.20) as usize),
    };
    let (hx0, hx1) = if hx1 <= hx0 {
        (cx0 + (gw * 0.25) as usize, cx1 - (gw * 0.25) as usize)
    } else {
        (hx0, hx1)
    };
    let uy0 = dy0 + (gh * 0.10) as usize;
    let uy1 = dy0 + (gh * 0.45) as usize;
    let ly0 = dy0 + (gh * 0.55) as usize;
    let ly1 = dy0 + (gh * 0.90) as usize;
    let mut zones: Vec<(usize, usize, usize, usize, u8)> = vec![
        (hx0, hx1, dy0, dy0 + (gh * 0.18) as usize, SEG_TOP),
        (hx0, hx1, dy0 + (gh * 0.82) as usize, dy1, SEG_BOTTOM),
        (hx0, hx1, dy0 + (gh * 0.41) as usize, dy0 + (gh * 0.59) as usize, SEG_MIDDLE),
    ];
    if let Some((zx0, zx1)) = rz {
        zones.push((zx0, zx1, uy0, uy1, SEG_UPPER_RIGHT));
        zones.push((zx0, zx1, ly0, ly1, SEG_LOWER_RIGHT));
    }
    if let Some((zx0, zx1)) = lz {
        zones.push((zx0, zx1, ly0, ly1, SEG_LOWER_LEFT));
        zones.push((zx0, zx1, uy0, uy1, SEG_UPPER_LEFT));
    }
    let mut pattern_bright = 0u8;
    let mut pattern_dark = 0u8;
    for (zx0, zx1, zy0, zy1, bit) in zones {
        if zx1 <= zx0 || zy1 <= zy0 || zx1 >= w {
            continue;
        }
        let mut sum = 0usize;
        let mut count = 0usize;
        for r in zy0..=zy1 {
            let row = &band_px[r * w..r * w + w];
            for x in zx0..=zx1 {
                sum += row[x] as usize;
                count += 1;
            }
        }
        if count == 0 {
            continue;
        }
        let m = sum as f32 / count as f32;
        if m - bg > delta {
            pattern_bright |= bit;
        }
        if bg - m > delta {
            pattern_dark |= bit;
        }
    }
    let mut best: Option<(u8, u32, u32)> = None;
    for p in [pattern_bright, pattern_dark] {
        if let Some(m) = match_digit(p) {
            let better = match &best {
                Some(b) => m.1 < b.1 || (m.1 == b.1 && m.2 > b.2),
                None => true,
            };
            if better {
                best = Some(m);
            }
        }
    }
    best
}

fn is_red_digit(color: &dyn ColorValue, cell: (u32, u32), y0: u32, y1: u32) -> bool {
    let (cx0, cx1) = (cell.0, cell.1);
    let mut r = 0u64;
    let mut g = 0u64;
    let mut b = 0u64;
    let mut n = 0u64;
    for y in y0..=y1 {
        for x in cx0..=cx1 {
            let (rr, gg, bb) = color.get_pixel_rgb(x, y);
            r += rr as u64;
            g += gg as u64;
            b += bb as u64;
            n += 1;
        }
    }
    if n == 0 {
        return false;
    }
    let (r, g, b) = (r as f32 / n as f32, g as f32 / n as f32, b as f32 / n as f32);
    r > g + 25.0 && r > b + 25.0
}
