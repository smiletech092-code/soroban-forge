/// Optional argument validation against contract spec for invoke.

use std::path::Path;
use soroban_forge_core::{ForgeError, Result};

/// Validate function name and arguments against contract spec.
pub fn validate_invoke_args(
    wasm_path: &Path,
    function_name: &str,
    args: &[String],
) -> Result<()> {
    let spec_json = extract_contract_spec(wasm_path)?;

    let functions = spec_json
        .get("functions")
        .and_then(|f| f.as_array())
        .ok_or_else(|| ForgeError::InvalidArgument("Invalid contract spec format".into()))?;

    let function = functions
        .iter()
        .find(|f| {
            f.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n == function_name)
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            let available_functions: Vec<String> = functions
                .iter()
                .filter_map(|f| f.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect();

            ForgeError::InvalidArgument(format!(
                "Function `{}` not found in contract. Available functions: {}",
                function_name,
                available_functions.join(", ")
            ))
        })?;

    let expected_inputs = function
        .get("inputs")
        .and_then(|i| i.as_array())
        .unwrap_or(&vec![]);

    validate_args(args, expected_inputs)?;

    Ok(())
}

fn validate_args(args: &[String], expected_inputs: &[serde_json::Value]) -> Result<()> {
    let required_args: Vec<String> = expected_inputs
        .iter()
        .filter_map(|input| input.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect();

    let provided_args: std::collections::HashSet<String> = args
        .iter()
        .filter(|arg| arg.starts_with("--"))
        .map(|arg| arg.trim_start_matches("--").to_string())
        .collect();

    for required in &required_args {
        if !provided_args.contains(required) {
            return Err(ForgeError::InvalidArgument(format!(
                "Missing required argument: --{}",
                required
            )));
        }
    }

    for provided in &provided_args {
        if !required_args.contains(provided) {
            return Err(ForgeError::InvalidArgument(format!(
                "Unknown argument: --{}. Expected: {}",
                provided,
                required_args.join(", ")
            )));
        }
    }

    Ok(())
}

fn extract_contract_spec(wasm_path: &Path) -> Result<serde_json::Value> {
    use std::process::Command;

    let output = Command::new("stellar")
        .args(&[
            "contract",
            "info",
            "interface",
            "--wasm",
            wasm_path.to_str().ok_or_else(|| ForgeError::Other("Invalid wasm path".into()))?,
            "--output",
            "json-formatted",
        ])
        .output()
        .map_err(|e| ForgeError::Other(format!("Failed to extract contract spec: {}", e)))?;

    if !output.status.success() {
        return Err(ForgeError::Other(
            "Failed to extract contract spec from wasm".into(),
        ));
    }

    let spec_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&spec_str).map_err(|e| {
        ForgeError::Other(format!("Failed to parse contract spec JSON: {}", e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_args() {
        let inputs = vec![
            serde_json::json!({"name": "to", "type": "Address"}),
            serde_json::json!({"name": "amount", "type": "i128"}),
        ];

        let args = vec!["--to".to_string(), "CBAT...".to_string(), "--amount".to_string(), "100".to_string()];

        assert!(validate_args(&args, &inputs).is_ok());
    }

    #[test]
    fn test_validate_args_missing_required() {
        let inputs = vec![
            serde_json::json!({"name": "to", "type": "Address"}),
            serde_json::json!({"name": "amount", "type": "i128"}),
        ];

        let args = vec!["--to".to_string(), "CBAT...".to_string()];

        assert!(validate_args(&args, &inputs).is_err());
    }

    #[test]
    fn test_validate_args_unknown_arg() {
        let inputs = vec![
            serde_json::json!({"name": "to", "type": "Address"}),
        ];

        let args = vec!["--to".to_string(), "CBAT...".to_string(), "--invalid".to_string(), "value".to_string()];

        assert!(validate_args(&args, &inputs).is_err());
    }
}
