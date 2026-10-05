/// Identity helpers for invoke: suggestions and validation.

use std::path::Path;
use soroban_forge_core::Result;

/// Suggest a close identity name when --source is not found.
pub fn suggest_identity(
    identity_dir: &Path,
    given_name: &str,
) -> Option<String> {
    let stored_identities = list_stored_identities(identity_dir).ok()?;

    if stored_identities.is_empty() {
        return None;
    }

    let mut closest = None;
    let mut min_distance = u32::MAX;

    for stored_name in stored_identities {
        let distance = levenshtein_distance(given_name, &stored_name);
        if distance < min_distance && distance <= 2 {
            min_distance = distance;
            closest = Some(stored_name);
        }
    }

    closest
}

/// List all stored identity names in the identity directory.
pub fn list_stored_identities(identity_dir: &Path) -> Result<Vec<String>> {
    let mut identities = Vec::new();

    if identity_dir.exists() {
        for entry in std::fs::read_dir(identity_dir)
            .map_err(|e| soroban_forge_core::ForgeError::io("reading identity directory")(e))?
        {
            let entry = entry.map_err(|e| soroban_forge_core::ForgeError::io("reading identity entry")(e))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    identities.push(name.to_string());
                }
            }
        }
    }

    identities.sort();
    Ok(identities)
}

/// Compute Levenshtein distance between two strings for fuzzy matching.
fn levenshtein_distance(s1: &str, s2: &str) -> u32 {
    let len1 = s1.len();
    let len2 = s2.len();
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i as u32;
    }
    for j in 0..=len2 {
        matrix[0][j] = j as u32;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1.chars().nth(i - 1) == s2.chars().nth(j - 1) { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                matrix[i - 1][j - 1] + cost,
            );
        }
    }

    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("alice", "alice"), 0);
        assert_eq!(levenshtein_distance("alice", "alice1"), 1);
        assert_eq!(levenshtein_distance("alice", "alices"), 1);
        assert_eq!(levenshtein_distance("alice", "alce"), 1);
        assert_eq!(levenshtein_distance("alice", "bob"), 5);
    }

    #[test]
    fn test_suggest_identity() {
        let temp_dir = tempfile::tempdir().unwrap();
        let id_dir = temp_dir.path();

        std::fs::create_dir_all(id_dir).unwrap();
        std::fs::write(id_dir.join("alice"), "").unwrap();
        std::fs::write(id_dir.join("bob"), "").unwrap();
        std::fs::write(id_dir.join("charlie"), "").unwrap();

        assert_eq!(suggest_identity(id_dir, "alce"), Some("alice".to_string()));
        assert_eq!(suggest_identity(id_dir, "alise"), Some("alice".to_string()));
        assert_eq!(suggest_identity(id_dir, "bbo"), Some("bob".to_string()));
    }
}
