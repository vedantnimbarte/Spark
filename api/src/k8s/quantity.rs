//! Parsing Kubernetes resource quantities.
//!
//! metrics-server reports CPU in nanocores ("12345n") and memory in binary
//! suffixes ("123456Ki"), but the units it chooses vary with magnitude. Getting
//! this wrong misreports usage by a factor of a thousand without failing, which
//! is why it is parsed here with tests rather than inline.

/// CPU quantity to millicores.
pub fn cpu_millicores(value: &str) -> Option<i64> {
    let value = value.trim();
    let (digits, scale) = split_suffix(value);
    let number: f64 = digits.parse().ok()?;

    let millis = match scale {
        // Bare value is whole cores.
        "" => number * 1000.0,
        "m" => number,
        "u" => number / 1000.0,
        "n" => number / 1_000_000.0,
        _ => return None,
    };
    Some(millis.round() as i64)
}

/// Memory quantity to bytes.
pub fn memory_bytes(value: &str) -> Option<i64> {
    let value = value.trim();
    let (digits, scale) = split_suffix(value);
    let number: f64 = digits.parse().ok()?;

    let multiplier: f64 = match scale {
        "" => 1.0,
        "Ki" => 1024.0,
        "Mi" => 1024.0 * 1024.0,
        "Gi" => 1024.0 * 1024.0 * 1024.0,
        "Ti" => 1024.0f64.powi(4),
        // Decimal suffixes are equally legal and mean powers of 1000.
        "k" => 1000.0,
        "M" => 1_000_000.0,
        "G" => 1_000_000_000.0,
        "T" => 1e12,
        _ => return None,
    };
    Some((number * multiplier).round() as i64)
}

fn split_suffix(value: &str) -> (&str, &str) {
    let end = value
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(value.len());
    value.split_at(end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_units_convert_to_millicores() {
        assert_eq!(cpu_millicores("2"), Some(2000));
        assert_eq!(cpu_millicores("500m"), Some(500));
        assert_eq!(cpu_millicores("1500000u"), Some(1500));
        // The unit metrics-server actually uses most of the time.
        assert_eq!(cpu_millicores("12345678n"), Some(12));
        assert_eq!(cpu_millicores("0"), Some(0));
    }

    #[test]
    fn memory_units_convert_to_bytes() {
        assert_eq!(memory_bytes("1024"), Some(1024));
        assert_eq!(memory_bytes("1Ki"), Some(1024));
        assert_eq!(memory_bytes("2Mi"), Some(2 * 1024 * 1024));
        assert_eq!(memory_bytes("1Gi"), Some(1024 * 1024 * 1024));
        // Binary and decimal suffixes differ and must not be conflated.
        assert_ne!(memory_bytes("1M"), memory_bytes("1Mi"));
        assert_eq!(memory_bytes("1M"), Some(1_000_000));
    }

    #[test]
    fn unknown_or_malformed_values_are_rejected_rather_than_guessed() {
        assert_eq!(cpu_millicores("abc"), None);
        assert_eq!(cpu_millicores("10x"), None);
        assert_eq!(memory_bytes(""), None);
        assert_eq!(memory_bytes("12Zi"), None);
    }
}
