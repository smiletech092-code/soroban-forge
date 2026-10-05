/// Repeat invoke functionality for calling the same function multiple times.

use soroban_forge_core::Result;

/// Result of a single repeated invocation.
#[derive(Debug, Clone)]
pub struct RepeatResult {
    pub call_number: u32,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// Configuration for repeating invocations.
pub struct RepeatConfig {
    pub times: u32,
    pub continue_on_error: bool,
}

/// Repeat a function call N times and collect results.
pub fn repeat_invoke<F>(
    config: RepeatConfig,
    mut invoke_fn: F,
) -> Result<Vec<RepeatResult>>
where
    F: FnMut() -> Result<String>,
{
    let mut results = Vec::new();

    for call_num in 1..=config.times {
        match invoke_fn() {
            Ok(output) => {
                results.push(RepeatResult {
                    call_number: call_num,
                    success: true,
                    output,
                    error: None,
                });
            }
            Err(e) => {
                let error_msg = e.to_string();
                results.push(RepeatResult {
                    call_number: call_num,
                    success: false,
                    output: String::new(),
                    error: Some(error_msg.clone()),
                });

                if !config.continue_on_error {
                    return Err(e);
                }
            }
        }
    }

    Ok(results)
}

/// Format repeat results for display.
pub fn format_repeat_results(results: &[RepeatResult]) -> String {
    let mut output = String::new();

    for result in results {
        output.push_str(&format!("Call #{}: ", result.call_number));

        if result.success {
            output.push_str("✓ SUCCESS\n");
            if !result.output.is_empty() {
                output.push_str("  Output: ");
                output.push_str(&result.output);
                if !result.output.ends_with('\n') {
                    output.push('\n');
                }
            }
        } else {
            output.push_str("✗ FAILED\n");
            if let Some(error) = &result.error {
                output.push_str(&format!("  Error: {}\n", error));
            }
        }
    }

    let success_count = results.iter().filter(|r| r.success).count();
    let total_count = results.len();

    output.push_str(&format!(
        "\nSummary: {}/{} calls succeeded",
        success_count, total_count
    ));

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repeat_invoke_success() {
        let config = RepeatConfig {
            times: 3,
            continue_on_error: false,
        };

        let mut call_count = 0;
        let invoke_fn = || {
            call_count += 1;
            Ok(format!("Result {}", call_count))
        };

        let results = repeat_invoke(config, invoke_fn).unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn test_repeat_invoke_with_error() {
        let config = RepeatConfig {
            times: 3,
            continue_on_error: false,
        };

        let mut call_count = 0;
        let invoke_fn = || {
            call_count += 1;
            if call_count == 2 {
                Err(soroban_forge_core::ForgeError::Other("Test error".into()))
            } else {
                Ok(format!("Result {}", call_count))
            }
        };

        let results = repeat_invoke(config, invoke_fn);
        assert!(results.is_err());
    }

    #[test]
    fn test_repeat_invoke_continue_on_error() {
        let config = RepeatConfig {
            times: 3,
            continue_on_error: true,
        };

        let mut call_count = 0;
        let invoke_fn = || {
            call_count += 1;
            if call_count == 2 {
                Err(soroban_forge_core::ForgeError::Other("Test error".into()))
            } else {
                Ok(format!("Result {}", call_count))
            }
        };

        let results = repeat_invoke(config, invoke_fn).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results.iter().filter(|r| r.success).count(), 2);
        assert_eq!(results.iter().filter(|r| !r.success).count(), 1);
    }

    #[test]
    fn test_format_repeat_results() {
        let results = vec![
            RepeatResult {
                call_number: 1,
                success: true,
                output: "Result 1".to_string(),
                error: None,
            },
            RepeatResult {
                call_number: 2,
                success: false,
                output: String::new(),
                error: Some("Network error".to_string()),
            },
        ];

        let formatted = format_repeat_results(&results);
        assert!(formatted.contains("Call #1: ✓ SUCCESS"));
        assert!(formatted.contains("Call #2: ✗ FAILED"));
        assert!(formatted.contains("1/2 calls succeeded"));
    }
}
