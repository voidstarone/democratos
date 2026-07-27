//! Dimension and pixel-count ceilings that defuse decompression bombs.

/// The largest pixel count a decoded image may have (40 megapixels).
///
/// A decompression ("pixel") bomb is a small file that declares an enormous
/// canvas: a few hundred KB of PNG can claim 60 000 × 60 000 pixels and cost
/// gigabytes the instant something decodes it. The defence is to read the
/// dimensions from the *header* and refuse before a pixel buffer is ever
/// allocated, which is why this bound exists separately from
/// [`MAX_UPLOAD_BYTES`](crate::MAX_UPLOAD_BYTES): file size says nothing about
/// decoded cost.
///
/// A `u64` because it is compared against `width * height` widened to avoid the
/// overflow that would otherwise turn this check into a no-op on hostile input.
pub const MAX_IMAGE_PIXELS: u64 = 40_000_000;

/// The largest width or height a decoded image may have (20 000 px).
///
/// Checked alongside [`MAX_IMAGE_PIXELS`] because the two catch different shapes:
/// the pixel budget alone would permit a 1 × 40 000 000 strip, which is cheap by
/// area but pathological for decoders that allocate per-row structures. A `u32`
/// to match the dimension type image decoders report.
pub const MAX_IMAGE_DIMENSION: u32 = 20_000;
