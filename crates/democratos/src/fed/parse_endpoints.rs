//! Split a comma-separated endpoint list, dropping empties.

/// Split a comma-separated endpoint list, dropping empties.
pub fn parse_endpoints(csv: &str) -> Vec<String> {
    csv.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
