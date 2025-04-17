/// Parses a human-readable file size (e.g., "1", "1B", "10M", "1.3G") into bytes.
pub(super) fn parse_file_size(size_str: &str) -> Option<usize> {
    let size_str = size_str.trim().to_uppercase();
    if size_str.is_empty() {
        return None;
    }

    // Check if the string ends with a unit (B, K, M, G, T)
    let (num_part, unit) = if size_str.ends_with('B') {
        (&size_str[..size_str.len() - 1], 'B')
    } else if let Some(last_char) = size_str.chars().last() {
        if last_char.is_alphabetic() {
            (&size_str[..size_str.len() - 1], last_char)
        } else {
            (size_str.as_str(), 'B') // Default to bytes if no unit
        }
    } else {
        (size_str.as_str(), 'B')
    };

    // Parse the numeric part
    let num = num_part.parse::<f64>().ok()?;
    if num < 0.0 {
        return None;
    }

    // Calculate bytes based on the unit
    let multiplier: usize = match unit {
        'B' => 1,
        'K' => 1024,
        'M' => 1024 * 1024,
        'G' => 1024 * 1024 * 1024,
        'T' => 1024 * 1024 * 1024 * 1024,
        _ => return None, // Unknown unit
    };

    Some((num * multiplier as f64) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_size_valid() {
        // Test cases with valid inputs
        assert_eq!(parse_file_size("1"), Some(1));
        assert_eq!(parse_file_size("1B"), Some(1));
        assert_eq!(parse_file_size("10K"), Some(10 * 1024));
        assert_eq!(
            parse_file_size("2.5M"),
            Some((2.5 * 1024.0 * 1024.0) as usize)
        );
        assert_eq!(parse_file_size("1G"), Some(1024 * 1024 * 1024));
        assert_eq!(
            parse_file_size("0.5T"),
            Some((0.5 * 1024.0 * 1024.0 * 1024.0 * 1024.0) as usize)
        );
        assert_eq!(parse_file_size(" 123  "), Some(123)); // Test trimming
        assert_eq!(parse_file_size("1k"), Some(1024)); // Case-insensitive
        assert_eq!(parse_file_size("1m"), Some(1024 * 1024)); // Case-insensitive
    }

    #[test]
    fn test_parse_file_size_invalid() {
        // Test cases with invalid inputs
        assert_eq!(parse_file_size(""), None); // Empty string
        assert_eq!(parse_file_size("ABC"), None); // Non-numeric
        assert_eq!(parse_file_size("1.2.3"), None); // Invalid numeric format
        assert_eq!(parse_file_size("1X"), None); // Unknown unit
        assert_eq!(parse_file_size("-1"), None); // Negative number (if not supported)
    }

    #[test]
    fn test_parse_file_size_edge_cases() {
        // Edge cases
        assert_eq!(parse_file_size("0"), Some(0)); // Zero value
        assert_eq!(parse_file_size("0B"), Some(0)); // Zero with unit
        assert_eq!(
            parse_file_size("999999999999T"),
            Some((999999999999.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0) as usize)
        );
    }
}
