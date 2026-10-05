/// Function discovery for invoke: listing callable functions before invoking.

use soroban_forge_core::{ForgeError, Result};
use std::process::Command;

/// List callable functions from a deployed contract.
pub fn list_contract_functions(
    contract_id: &str,
    network: Option<&str>,
    rpc_url: Option<&str>,
) -> Result<Vec<ContractFunction>> {
    let spec_json = fetch_contract_spec(contract_id, network, rpc_url)?;

    let functions = spec_json
        .get("functions")
        .and_then(|f| f.as_array())
        .ok_or_else(|| ForgeError::Other("Invalid contract spec format".into()))?;

    let mut result = Vec::new();

    for func in functions {
        let name = func
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| ForgeError::Other("Missing function name in spec".into()))?
            .to_string();

        let inputs = func
            .get("inputs")
            .and_then(|i| i.as_array())
            .unwrap_or(&vec![]);

        let outputs = func
            .get("outputs")
            .and_then(|o| o.as_array())
            .unwrap_or(&vec![]);

        let inputs_str = format_inputs(inputs);
        let outputs_str = format_outputs(outputs);

        result.push(ContractFunction {
            name,
            inputs: inputs_str,
            outputs: outputs_str,
        });
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct ContractFunction {
    pub name: String,
    pub inputs: String,
    pub outputs: String,
}

impl ContractFunction {
    pub fn signature(&self) -> String {
        format!("{} ({}) -> {}", self.name, self.inputs, self.outputs)
    }
}

fn fetch_contract_spec(
    contract_id: &str,
    network: Option<&str>,
    rpc_url: Option<&str>,
) -> Result<serde_json::Value> {
    let mut args = vec!["contract", "info", "interface", "--id", contract_id, "--output", "json-formatted"];

    let network_str = network.map(|n| n.to_string());
    let rpc_url_str = rpc_url.map(|u| u.to_string());

    let network_owned;
    let rpc_owned;

    if let Some(ref net) = network_str {
        network_owned = net.clone();
        args.push("--network");
        args.push(&network_owned);
    } else if let Some(ref url) = rpc_url_str {
        rpc_owned = url.clone();
        args.push("--rpc-url");
        args.push(&rpc_owned);
    }

    let output = Command::new("stellar")
        .args(&args)
        .output()
        .map_err(|e| ForgeError::Other(format!("Failed to fetch contract spec: {}", e)))?;

    if !output.status.success() {
        return Err(ForgeError::Other(
            "Failed to fetch contract spec from network".into(),
        ));
    }

    let spec_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&spec_str)
        .map_err(|e| ForgeError::Other(format!("Failed to parse contract spec: {}", e)))
}

fn format_inputs(inputs: &[serde_json::Value]) -> String {
    inputs
        .iter()
        .filter_map(|input| {
            let name = input.get("name").and_then(|n| n.as_str())?;
            let ty = input.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
            Some(format!("{}: {}", name, ty))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_outputs(outputs: &[serde_json::Value]) -> String {
    if outputs.is_empty() {
        return "void".to_string();
    }

    outputs
        .iter()
        .filter_map(|output| output.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format a list of functions for display.
pub fn format_function_list(functions: &[ContractFunction]) -> String {
    let mut output = String::from("Callable functions:\n\n");

    for func in functions {
        output.push_str(&format!("  {}\n", func.signature()));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_function_signature() {
        let func = ContractFunction {
            name: "transfer".to_string(),
            inputs: "to: Address, amount: i128".to_string(),
            outputs: "bool".to_string(),
        };

        assert_eq!(func.signature(), "transfer (to: Address, amount: i128) -> bool");
    }

    #[test]
    fn test_format_function_list() {
        let functions = vec![
            ContractFunction {
                name: "transfer".to_string(),
                inputs: "to: Address, amount: i128".to_string(),
                outputs: "bool".to_string(),
            },
            ContractFunction {
                name: "balance_of".to_string(),
                inputs: "account: Address".to_string(),
                outputs: "i128".to_string(),
            },
        ];

        let formatted = format_function_list(&functions);
        assert!(formatted.contains("transfer"));
        assert!(formatted.contains("balance_of"));
    }
}
