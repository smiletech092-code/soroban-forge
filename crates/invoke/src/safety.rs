/// Safety features for invoke: confirmations and error classification.

use std::io::Write;
use soroban_forge_core::Result;

/// Check if network is mainnet and prompt for confirmation if needed.
pub fn confirm_mainnet_transaction(
    network: &Option<String>,
    rpc_url: &Option<String>,
    network_passphrase: &Option<String>,
    skip_confirmation: bool,
) -> Result<()> {
    let is_mainnet = is_mainnet_network(network, rpc_url, network_passphrase);

    if is_mainnet && !skip_confirmation {
        prompt_mainnet_confirmation()?;
    }

    Ok(())
}

/// Determine if the network is mainnet based on network name, RPC URL, or passphrase.
fn is_mainnet_network(
    network: &Option<String>,
    rpc_url: &Option<String>,
    network_passphrase: &Option<String>,
) -> bool {
    if let Some(net) = network {
        if net.to_lowercase() == "mainnet" || net.to_lowercase() == "public" {
            return true;
        }
    }

    if let Some(url) = rpc_url {
        if url.contains("mainnet") || url.contains("public-rpc") || url.contains("soroban-rpc.stellar.org") {
            return true;
        }
    }

    if let Some(passphrase) = network_passphrase {
        if passphrase == "Public Global Stellar Network ; September 2015" {
            return true;
        }
    }

    false
}

/// Prompt user for confirmation before invoking on mainnet.
fn prompt_mainnet_confirmation() -> Result<()> {
    print!("⚠️  WARNING: This will invoke a function on MAINNET with real costs and consequences.\n");
    print!("Type 'yes' to confirm, or press Ctrl+C to cancel: ");
    std::io::stdout().flush().ok();

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| soroban_forge_core::ForgeError::Other(format!("Failed to read user input: {}", e)))?;

    if input.trim().to_lowercase() != "yes" {
        return Err(soroban_forge_core::ForgeError::Other(
            "Mainnet invocation cancelled by user".into(),
        ));
    }

    Ok(())
}

/// Classification of invoke failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvokeFailureType {
    /// The contract executed but panicked/reverted.
    ContractPanic,
    /// The stellar CLI process itself failed (argument error, missing contract, etc.).
    CliFailure,
    /// Network or RPC connectivity error.
    NetworkError,
    /// Unknown/uncategorized failure.
    Other,
}

impl InvokeFailureType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvokeFailureType::ContractPanic => "contract_panic",
            InvokeFailureType::CliFailure => "cli_failure",
            InvokeFailureType::NetworkError => "network_error",
            InvokeFailureType::Other => "other",
        }
    }
}

/// Classify an invoke failure based on stderr output and exit code.
pub fn classify_invoke_failure(stderr: &str, exit_code: Option<i32>) -> InvokeFailureType {
    if stderr.contains("panic") || stderr.contains("reverted") || stderr.contains("error code") {
        return InvokeFailureType::ContractPanic;
    }

    if stderr.contains("Connection refused") || stderr.contains("connection timeout") || stderr.contains("network") {
        return InvokeFailureType::NetworkError;
    }

    if stderr.contains("unknown flag") || stderr.contains("unexpected argument") || stderr.contains("invalid syntax") {
        return InvokeFailureType::CliFailure;
    }

    if exit_code == Some(1) && stderr.contains("contract") {
        return InvokeFailureType::ContractPanic;
    }

    InvokeFailureType::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_mainnet_network() {
        assert!(is_mainnet_network(&Some("mainnet".to_string()), &None, &None));
        assert!(is_mainnet_network(&Some("public".to_string()), &None, &None));
        assert!(!is_mainnet_network(&Some("testnet".to_string()), &None, &None));

        assert!(is_mainnet_network(
            &None,
            &Some("https://soroban-rpc.stellar.org".to_string()),
            &None
        ));

        let mainnet_passphrase = "Public Global Stellar Network ; September 2015";
        assert!(is_mainnet_network(&None, &None, &Some(mainnet_passphrase.to_string())));
    }

    #[test]
    fn test_classify_invoke_failure() {
        assert_eq!(
            classify_invoke_failure("contract panicked with error code 10", None),
            InvokeFailureType::ContractPanic
        );

        assert_eq!(
            classify_invoke_failure("unknown flag '--invalid'", None),
            InvokeFailureType::CliFailure
        );

        assert_eq!(
            classify_invoke_failure("Connection refused", None),
            InvokeFailureType::NetworkError
        );
    }
}
