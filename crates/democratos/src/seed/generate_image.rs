//! Generate a small self-contained SVG placeholder image as a `data:` URI.

use domain::Media;

/// Generate a small self-contained coloured SVG placeholder as a `data:` URI, so
/// seed images render inline with no media store, external host, or image-codec
/// dependency — yet still exercise the multi-media post UI end to end.
pub(crate) fn generate_image(caption: &str, seed: u32) -> Media {
    let hue = seed.wrapping_mul(47) % 360;
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='480' height='320'>\
         <rect width='480' height='320' fill='hsl({hue},60%,55%)'/>\
         <text x='24' y='300' font-family='sans-serif' font-size='22' fill='white'>{}</text>\
         </svg>",
        xml_escape(caption)
    );
    Media::image(
        format!("data:image/svg+xml,{}", percent_encode(&svg)),
        caption.to_string(),
    )
}

/// Minimal XML text escaping for the caption baked into the SVG.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Percent-encode a string for embedding in a `data:` URI (RFC 3986 unreserved
/// set passes through; everything else is `%XX`-escaped).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
