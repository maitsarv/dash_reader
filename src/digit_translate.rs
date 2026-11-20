pub const SEG_TOP: u8 = 1 << 0;
pub const SEG_UPPER_RIGHT: u8 = 1 << 1;
pub const SEG_LOWER_RIGHT: u8 = 1 << 2;
pub const SEG_BOTTOM: u8 = 1 << 3;
pub const SEG_LOWER_LEFT: u8 = 1 << 4;
pub const SEG_UPPER_LEFT: u8 = 1 << 5;
pub const SEG_MIDDLE: u8 = 1 << 6;

pub const DIGIT_SEGMENTS: [(u8, u8); 10] = [
    (0, SEG_TOP | SEG_UPPER_RIGHT | SEG_LOWER_RIGHT | SEG_BOTTOM | SEG_LOWER_LEFT | SEG_UPPER_LEFT),
    (1, SEG_UPPER_RIGHT | SEG_LOWER_RIGHT),
    (2, SEG_TOP | SEG_UPPER_RIGHT | SEG_MIDDLE | SEG_LOWER_LEFT | SEG_BOTTOM),
    (3, SEG_TOP | SEG_UPPER_RIGHT | SEG_MIDDLE | SEG_LOWER_RIGHT | SEG_BOTTOM),
    (4, SEG_UPPER_LEFT | SEG_MIDDLE | SEG_UPPER_RIGHT | SEG_LOWER_RIGHT),
    (5, SEG_TOP | SEG_UPPER_LEFT | SEG_MIDDLE | SEG_LOWER_RIGHT | SEG_BOTTOM),
    (6, SEG_TOP | SEG_UPPER_LEFT | SEG_MIDDLE | SEG_LOWER_LEFT | SEG_LOWER_RIGHT | SEG_BOTTOM),
    (7, SEG_TOP | SEG_UPPER_RIGHT | SEG_LOWER_RIGHT),
    (8, SEG_TOP | SEG_UPPER_RIGHT | SEG_LOWER_RIGHT | SEG_BOTTOM | SEG_LOWER_LEFT | SEG_UPPER_LEFT | SEG_MIDDLE),
    (9, SEG_TOP | SEG_UPPER_RIGHT | SEG_LOWER_RIGHT | SEG_BOTTOM | SEG_UPPER_LEFT | SEG_MIDDLE),
];

pub fn segment_distance(a: u8, b: u8) -> u32 {
    (a ^ b).count_ones()
}

pub fn match_digit(pattern: u8) -> Option<(u8, u32, u32)> {
    let mut best_digit = 10u8;
    let mut best = u32::MAX;
    let mut second = u32::MAX;
    for &(d, seg) in DIGIT_SEGMENTS.iter() {
        let dist = segment_distance(pattern, seg);
        if dist < best {
            second = best;
            best = dist;
            best_digit = d;
        } else if dist < second {
            second = dist;
        }
    }
    if best_digit > 9 {
        return None;
    }
    if best == 0 || (best == 1 && second >= 3) {
        return Some((best_digit, best, second));
    }
    None
}
