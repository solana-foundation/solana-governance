use flate2::read::GzDecoder;
use std::io::{self, Read};

// ENV config for maximum allowed size (in bytes) of a decompressed snapshot payload.
// This prevents zip-bomb style decompression from exhausting memory.
pub const DEFAULT_MAX_DECOMPRESSED_SNAPSHOT_BYTES: usize = 256 * 1024 * 1024; // 256 MiB

pub fn max_snapshot_bytes() -> usize {
    if let Ok(mb_str) = std::env::var("GOV_V1_MAX_SNAPSHOT_MB") {
        if let Ok(mb) = mb_str.parse::<usize>() {
            return mb.saturating_mul(1024 * 1024);
        }
    }
    DEFAULT_MAX_DECOMPRESSED_SNAPSHOT_BYTES
}

pub fn read_all_with_limit<R: Read>(mut reader: R, max_size: usize) -> io::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(std::cmp::min(max_size, 64 * 1024));
    let mut chunk = [0u8; 8192];
    let mut total = 0usize;
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        total += n;
        if total > max_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "decompressed size limit exceeded",
            ));
        }
        out.extend_from_slice(&chunk[..n]);
    }
    Ok(out)
}

pub fn decompress_gzip_with_limit<R: Read>(reader: R, max_size: usize) -> io::Result<Vec<u8>> {
    let decoder = GzDecoder::new(reader);
    read_all_with_limit(decoder, max_size)
}


