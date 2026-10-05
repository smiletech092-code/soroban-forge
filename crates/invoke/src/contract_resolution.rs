/// Contract name resolution for invoke: aliases and deployments file.

use std::collections::HashMap;
use std::path::Path;
use soroban_forge_core::{ForgeError, Result};

const CONTRACT_ID_LEN: usize = 56;

/// Resolve a contract identifier (name or contract ID) to actual contract ID.
pub fn resolve_contract_id(
    identifier: &str,
    deployments_file: Option<&Path>,
    aliases: &HashMap<String, String>,
) -> Result<String> {
    if is_valid_contract_id(identifier) {
        return Ok(identifier.to_string());
    }

    if let Some(id) = aliases.get(identifier) {
        return Ok(id.clone());
    }

    if let Some(path) = deployments_file {
        if let Ok(id) = resolve_from_deployments_file(path, identifier) {
            return Ok(id);
        }
    }

    Err(ForgeError::InvalidArgument(format!(
        "Unknown contract identifier: `{}`\n\
         Please provide a valid 56-character contract ID starting with 'C', or register an alias",
        identifier
    )))
}

fn is_valid_contract_id(id: &str) -> bool {
    id.len() == CONTRACT_ID_LEN && id.starts_with('C') && id.chars().skip(1).all(|c| matches!(c, 'A'..='Z' | '2'..='7'))
}

fn resolve_from_deployments_file(path: &Path, name: &str) -> Result<String> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        ForgeError::io(format!("reading deployments file {}", path.display()))(e)
    })?;

    let deployments: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        ForgeError::Config {
            path: path.to_path_buf(),
            message: format!("Invalid JSON: {}", e),
        }
    })?;

    if let Some(id) = deployments.get(name).and_then(|v| v.get("id")).and_then(|v| v.as_str()) {
        return Ok(id.to_string());
    }

    Err(ForgeError::InvalidArgument(format!(
        "Contract `{}` not found in deployments file",
        name
    )))
}

/// List available contract names from aliases and deployments file.
pub fn list_available_contracts(
    deployments_file: Option<&Path>,
    aliases: &HashMap<String, String>,
) -> Vec<String> {
    let mut contracts = Vec::new();

    contracts.extend(aliases.keys().cloned());

    if let Some(path) = deployments_file {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(deployments) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = deployments.as_object() {
                    contracts.extend(obj.keys().cloned());
                }
            }
        }
    }

    contracts.sort();
    contracts.dedup();
    contracts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_contract_id() {
        assert!(is_valid_contract_id("CBAT7ZKW4OQQKSKEWLH4JTGX4M3SWYX2O6OG5LXKMPPFMQO5FJVXV5J2"));
        assert!(!is_valid_contract_id("INVALID"));
        assert!(!is_valid_contract_id("cbat7zkw4oqqkskewlh4jtgx4m3swyx2o6og5lxkmppfmqo5fjvxv5j2"));
    }

    #[test]
    fn test_resolve_contract_id_valid() {
        let id = "CBAT7ZKW4OQQKSKEWLH4JTGX4M3SWYX2O6OG5LXKMPPFMQO5FJVXV5J2";
        let result = resolve_contract_id(id, None, &HashMap::new()).unwrap();
        assert_eq!(result, id);
    }

    #[test]
    fn test_resolve_contract_id_alias() {
        let mut aliases = HashMap::new();
        aliases.insert("my-token".to_string(), "CBAT7ZKW4OQQKSKEWLH4JTGX4M3SWYX2O6OG5LXKMPPFMQO5FJVXV5J2".to_string());

        let result = resolve_contract_id("my-token", None, &aliases).unwrap();
        assert_eq!(result, "CBAT7ZKW4OQQKSKEWLH4JTGX4M3SWYX2O6OG5LXKMPPFMQO5FJVXV5J2");
    }

    #[test]
    fn test_resolve_contract_id_unknown() {
        let result = resolve_contract_id("unknown", None, &HashMap::new());
        assert!(result.is_err());
    }
}
