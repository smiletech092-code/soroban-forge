/// Version checking for stellar-cli minimum required version.

use soroban_forge_core::ForgeError::Other;
use soroban_forge_core::Result;

const MINIMUM_STELLAR_CLI_VERSION: &str = "27.0.0";

/// Check that stellar-cli is installed and meets minimum version requirement.
pub fn check_stellar_cli_version() -> Result<()> {
    let output = std::process::Command::new("stellar")
        .arg("--version")
        .output()
        .map_err(|_| Other("Failed to run `stellar --version`; ensure stellar-cli is installed".into()))?;

    if !output.status.success() {
        return Err(Other("stellar-cli version check failed".into()));
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = parse_stellar_version(&version_str)
        .ok_or_else(|| {
            Other(format!(
                "Could not parse stellar-cli version from: {}\nPlease ensure stellar-cli is installed and working correctly",
                version_str.trim()
            ))
        })?;

    let min_version = parse_stellar_version(MINIMUM_STELLAR_CLI_VERSION)
        .ok_or_else(|| Other("Internal error: could not parse minimum version".into()))?;

    if version < min_version {
        return Err(Other(format!(
            "stellar-cli version {} is too old; please upgrade to {} or later (current: {})",
            version_str.trim(),
            MINIMUM_STELLAR_CLI_VERSION,
            version_str.trim()
        )));
    }

    Ok(())
}

fn parse_stellar_version(version_str: &str) -> Option<(u32, u32, u32)> {
    let trimmed = version_str.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().next()?.split('.').collect();

    if parts.len() >= 3 {
        let major = parts[0].parse::<u32>().ok()?;
        let minor = parts[1].parse::<u32>().ok()?;
        let patch = parts[2].parse::<u32>().ok()?;
        Some((major, minor, patch))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stellar_version() {
        assert_eq!(parse_stellar_version("27.0.0"), Some((27, 0, 0)));
        assert_eq!(parse_stellar_version("26.5.2"), Some((26, 5, 2)));
        assert_eq!(parse_stellar_version("stellar-cli 28.1.0"), Some((28, 1, 0)));
        assert_eq!(parse_stellar_version("invalid"), None);
    }

    #[test]
    fn test_version_comparison() {
        let min = parse_stellar_version(MINIMUM_STELLAR_CLI_VERSION).unwrap();
        assert!(parse_stellar_version("27.0.0").unwrap() >= min);
        assert!(parse_stellar_version("28.0.0").unwrap() >= min);
        assert!(!(parse_stellar_version("26.9.9").unwrap() >= min));
    }
}
