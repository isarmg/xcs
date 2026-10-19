//! Deterministic application icon rendered from Rust geometry.
use anyhow::Result;
use std::{fs, io::Write, path::Path};

pub fn generate(root: &Path) -> Result<()> {
    const SIZE: u32 = 128;
    let mut pixels = Vec::new();
    for y in 0..SIZE as i32 {
        pixels.push(0);
        for x in 0..SIZE as i32 {
            let mut color = [19, 105, 79, 255];
            if !(8..120).contains(&x) || !(8..120).contains(&y) {
                color = [0, 0, 0, 0];
            }
            let folder = (28..100).contains(&x) && (46..91).contains(&y)
                || (28..61).contains(&x) && (36..55).contains(&y);
            if folder {
                color = [227, 244, 235, 255];
            }
            if (45..84).contains(&x) && (62..67).contains(&y) {
                color = [19, 105, 79, 255];
            }
            for (cx, cy) in [(44, 64), (64, 64), (84, 64)] {
                if (x - cx) * (x - cx) + (y - cy) * (y - cy) < 30 {
                    color = [19, 105, 79, 255];
                }
            }
            pixels.extend_from_slice(&color);
        }
    }
    let mut image = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend_from_slice(&SIZE.to_be_bytes());
    header.extend_from_slice(&SIZE.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut image, b"IHDR", &header);
    let mut compressed = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    compressed.write_all(&pixels)?;
    chunk(&mut image, b"IDAT", &compressed.finish()?);
    chunk(&mut image, b"IEND", &[]);
    let icons = root.join("apps/host/icons");
    fs::create_dir_all(&icons)?;
    fs::write(icons.join("icon.png"), &image)?;
    let mut ico = vec![0, 0, 1, 0, 1, 0, 128, 128, 0, 0, 1, 0, 32, 0];
    ico.extend_from_slice(&(image.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&image);
    fs::write(icons.join("icon.ico"), ico)?;
    let mut icns = b"icns".to_vec();
    icns.extend_from_slice(&((image.len() + 16) as u32).to_be_bytes());
    icns.extend_from_slice(b"ic07");
    icns.extend_from_slice(&((image.len() + 8) as u32).to_be_bytes());
    icns.extend_from_slice(&image);
    fs::write(icons.join("icon.icns"), icns)?;
    Ok(())
}
fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(data);
    let mut crc = crc32fast::Hasher::new();
    crc.update(kind);
    crc.update(data);
    output.extend_from_slice(&crc.finalize().to_be_bytes());
}
