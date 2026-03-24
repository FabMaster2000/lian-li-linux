//! Rust FFI bindings for the vendored tinyuz compression library.
//!
//! tinyuz is an LZ77 variant designed for embedded systems.
//! The wireless fan firmware uses it to decompress RGB frame data.

use std::os::raw::c_uchar;

extern "C" {
    fn tuz_compress_mem(
        input: *const c_uchar,
        input_len: usize,
        output: *mut c_uchar,
        output_capacity: usize,
        dict_size: usize,
    ) -> usize;

    fn tuz_max_compressed_size(input_len: usize) -> usize;

    fn tuz_decompress_mem_wrapper(
        input: *const c_uchar,
        input_len: usize,
        output: *mut c_uchar,
        output_capacity: usize,
    ) -> usize;
}

/// Default dictionary size (4KB).
const DICT_SIZE_4K: usize = 4096;

/// Compress data using tinyuz with a 4KB dictionary.
///
/// Returns the compressed bytes, or an error if compression fails.
pub fn compress(input: &[u8]) -> anyhow::Result<Vec<u8>> {
    if input.is_empty() {
        anyhow::bail!("tinyuz: cannot compress empty input");
    }

    let max_size = unsafe { tuz_max_compressed_size(input.len()) };
    let mut output = vec![0u8; max_size];

    let compressed_len = unsafe {
        tuz_compress_mem(
            input.as_ptr(),
            input.len(),
            output.as_mut_ptr(),
            output.len(),
            DICT_SIZE_4K,
        )
    };

    if compressed_len == 0 {
        anyhow::bail!("tinyuz: compression failed (returned 0)");
    }

    output.truncate(compressed_len);
    Ok(output)
}

/// Decompress tinyuz-compressed data.
///
/// `output_capacity` should be the expected decompressed size
/// (e.g. `total_frames * led_num * 3` for RGB frame data).
///
/// Returns the decompressed bytes, or an error if decompression fails.
pub fn decompress(input: &[u8], output_capacity: usize) -> anyhow::Result<Vec<u8>> {
    if input.is_empty() {
        anyhow::bail!("tinyuz: cannot decompress empty input");
    }
    if output_capacity == 0 {
        anyhow::bail!("tinyuz: output_capacity must be > 0");
    }

    let mut output = vec![0u8; output_capacity];

    let decompressed_len = unsafe {
        tuz_decompress_mem_wrapper(
            input.as_ptr(),
            input.len(),
            output.as_mut_ptr(),
            output.len(),
        )
    };

    if decompressed_len == 0 {
        anyhow::bail!("tinyuz: decompression failed (returned 0)");
    }

    output.truncate(decompressed_len);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_solid_color() {
        // 20 LEDs, all red — should compress well
        let mut rgb_data = Vec::new();
        for _ in 0..20 {
            rgb_data.extend_from_slice(&[255, 0, 0]);
        }

        let compressed = compress(&rgb_data).expect("compression should succeed");
        assert!(!compressed.is_empty());
        // Solid color should compress smaller than original
        assert!(compressed.len() < rgb_data.len());
    }

    #[test]
    fn compress_gradient() {
        // Gradient data — less compressible
        let mut rgb_data = Vec::new();
        for i in 0..80u8 {
            rgb_data.extend_from_slice(&[i, i, i]);
        }

        let compressed = compress(&rgb_data).expect("compression should succeed");
        assert!(!compressed.is_empty());
    }

    #[test]
    fn round_trip_solid_color() {
        // Compress then decompress — output must match input
        let mut rgb_data = Vec::new();
        for _ in 0..44 {
            rgb_data.extend_from_slice(&[94, 199, 255]);
        }

        let compressed = compress(&rgb_data).expect("compress");
        let decompressed =
            decompress(&compressed, rgb_data.len()).expect("decompress");
        assert_eq!(decompressed, rgb_data);
    }

    #[test]
    fn round_trip_meteor_frames() {
        // Simulate 52 frames × 88 LEDs like a real Meteor transfer
        let mut rgb_data = Vec::with_capacity(52 * 88 * 3);
        for frame in 0..52u8 {
            for led in 0..88u8 {
                let brightness = ((frame as u16 + led as u16) % 256) as u8;
                rgb_data.extend_from_slice(&[0, brightness / 2, brightness]);
            }
        }

        let compressed = compress(&rgb_data).expect("compress");
        let decompressed =
            decompress(&compressed, rgb_data.len()).expect("decompress");
        assert_eq!(decompressed, rgb_data);
    }
}
