//! A difference hash (dHash) for near-duplicate image matching.

use image::DynamicImage;

/// Compute the 64-bit **difference hash** (dHash) of an image: reduce to a 9×8
/// greyscale, then set one bit per row-adjacent pixel pair according to whether
/// the left pixel is brighter than its right neighbour. The result is robust to
/// rescaling, mild compression, and colour shifts, so two visually similar images
/// hash close together (small Hamming distance) even when their bytes differ.
///
/// This is what lets the scanner catch a re-encoded or resized copy of a known-bad
/// image, not only a byte-identical one. It is a *matching* aid against a curated
/// list — never a classifier that judges unknown images.
pub fn dhash(image: &DynamicImage) -> u64 {
    // 9 wide × 8 tall greyscale → 8 comparisons per row × 8 rows = 64 bits.
    let small = image
        .resize_exact(9, 8, image::imageops::FilterType::Triangle)
        .into_luma8();
    let mut hash: u64 = 0;
    let mut bit = 0u32;
    for y in 0..8u32 {
        for x in 0..8u32 {
            let left = small.get_pixel(x, y)[0];
            let right = small.get_pixel(x + 1, y)[0];
            if left > right {
                hash |= 1u64 << bit;
            }
            bit += 1;
        }
    }
    hash
}

/// The Hamming distance between two dHashes — the number of differing bits. `0`
/// means the fingerprints are identical; small values mean visually similar.
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbImage};

    fn solid(w: u32, h: u32, v: u8) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_pixel(w, h, image::Rgb([v, v, v])))
    }

    #[test]
    fn identical_images_hash_identically() {
        assert_eq!(dhash(&solid(32, 32, 128)), dhash(&solid(64, 64, 128)));
    }

    #[test]
    fn a_textured_image_differs_from_a_flat_field() {
        // Alternating bright/dark columns produce left>right transitions (unlike a
        // flat field or a smooth gradient, both of which dHash to all zeros).
        let mut pattern = RgbImage::new(18, 16);
        for (x, _y, p) in pattern.enumerate_pixels_mut() {
            let v = if x % 2 == 0 { 240 } else { 10 };
            *p = image::Rgb([v, v, v]);
        }
        let d = hamming_distance(
            dhash(&DynamicImage::ImageRgb8(pattern)),
            dhash(&solid(16, 16, 0)),
        );
        assert!(d > 0, "a textured image should not match a flat field");
    }

    #[test]
    fn hamming_counts_differing_bits() {
        assert_eq!(hamming_distance(0b1010, 0b0011), 2);
        assert_eq!(hamming_distance(u64::MAX, u64::MAX), 0);
    }
}
