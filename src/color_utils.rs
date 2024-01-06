// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

pub fn hex_to_rgb(hex: &str) -> Option<[f32; 3]> {
    if hex.len() != 7 {
        return None;
    }

    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    return Some([r as f32 / 256.0, g as f32 / 256.0, b as f32 / 256.0]);
}

pub fn rgb_to_hex(rgb: &[f32; 3]) -> String {
    let rgb = rgb.map(|x| x.max(0.0).min(1.0));
    let r = (256.0 * rgb[0]) as u8;
    let g = (256.0 * rgb[1]) as u8;
    let b = (256.0 * rgb[2]) as u8;
    let hex = format!("{:02x}{:02x}{:02x}", r, g, b);
    return format!("#{}", hex);
}
