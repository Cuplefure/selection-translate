use std::fs;
use std::path::Path;

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn chunk(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(tag);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    out
}

fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut pos = 0usize;
    while pos < raw.len() {
        let end = (pos + 65535).min(raw.len());
        let block = &raw[pos..end];
        out.push(u8::from(end == raw.len()));
        out.extend_from_slice(&(block.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        out.extend_from_slice(block);
        pos = end;
    }
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in raw {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    out.extend_from_slice(&((b << 16) | a).to_be_bytes());
    out
}

fn make_png(size: usize) -> Vec<u8> {
    let mut raw = Vec::with_capacity((size * 4 + 1) * size);
    for y in 0..size {
        raw.push(0);
        for x in 0..size {
            let fx = x as f64 / size as f64;
            let fy = y as f64 / size as f64;
            let bubble = ((0.20..=0.80).contains(&fx) && (0.24..=0.62).contains(&fy))
                || ((0.34..=0.50).contains(&fx) && (0.62..=0.76).contains(&fy));
            let (r, g, b) = if bubble { (255, 255, 255) } else { (83, 74, 183) };
            raw.extend_from_slice(&[r, g, b, 255]);
        }
    }
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(size as u32).to_be_bytes());
    ihdr.extend_from_slice(&(size as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend_from_slice(&chunk(b"IHDR", &ihdr));
    png.extend_from_slice(&chunk(b"IDAT", &zlib_stored(&raw)));
    png.extend_from_slice(&chunk(b"IEND", &[]));
    png
}

/// Builds a version 3.00 ICO with a PNG-compressed image.
///
/// MSVC's `rc.exe` (used by tauri-winres on Windows) rejects an ICO whose
/// `ICONDIR.idReserved` is non-zero with `RC2175: not in 3.00 format`.
/// Embedding a complete PNG stream also avoids all DIB/AND-mask pitfalls of
/// a hand-written BMP-based icon entry, and matches the format produced by
/// Tauri's own `tauri icon` command.
fn make_ico() -> Vec<u8> {
    const S: usize = 32;
    let png = make_png(S);

    // ICONDIR: reserved must be 0, type 1 = icon, one image follows.
    let mut ico = Vec::with_capacity(22 + png.len());
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());

    // ICONDIRENTRY: 32x32, 256 colors marker 0, reserved 0, 1 plane, 32 bpp.
    ico.extend_from_slice(&[S as u8, S as u8, 0, 0]);
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());

    // Size and offset of the embedded PNG stream (ICONDIR + ICONDIRENTRY = 22).
    ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&png);
    ico
}

fn main() {
    let icons = Path::new("icons");
    fs::create_dir_all(icons).expect("failed to create icons directory");
    let png = make_png(256);
    fs::write(icons.join("icon.png"), &png).expect("failed to write icon.png");
    fs::write(icons.join("icon.ico"), make_ico()).expect("failed to write icon.ico");
    tauri_build::build()
}
