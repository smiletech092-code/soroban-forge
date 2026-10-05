# MSRV Validation Bugfix - Technical Design

## Overview

The `--msrv` flag in the `ci-init` command currently accepts any string value without validation, allowing invalid Rust version formats to be written into CI workflow files. This design formalizes the bug condition, specifies the fix implementation strategy, and defines a comprehensive testing approach to ensure validation fails early with clear error messages while preserving all existing correct behavior.

## Glossary

- **Bug_Condition (C)**: The condition that triggers the bug - when `--msrv` receives a string that does not match the `major.minor` or `major.minor.patch` format
- **Property (P)**: The desired behavior - invalid version strings are rejected with a clear error message before any files are written
- **Preservation**: Existing behavior for valid version strings, default MSRV, and all other flags that must remain unchanged
- **MSRV Version Format**: Either `major.minor` (e.g., `1.84`) or `major.minor.patch` (e.g., `1.84.0`) where each component is a sequence of decimal digits
- **GenerateOptions struct**: The Rust struct in `crates/ci-presets/src/lib.rs` that holds `--msrv` as an `Option<String>` field
- **generate() function**: The function in `crates/ci-presets/src/lib.rs` that processes GenerateOptions and writes CI workflow files to disk
- **DEFAULT_MSRV**: The default MSRV value (`"1.84"`) used when `--msrv` is not provided

## Bug Details

### Bug Condition

The bug manifests when a user passes `--msrv` with a string value that does not conform to the Rust version format specification. The `generate()` function in `crates/ci-presets/src/lib.rs` currently accepts any string and directly inserts it into template variables without validation. This allows invalid versions like `latest`, `1.84.0.1`, `v1.84`, or `1` to be written into the generated CI workflow files.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type Option<String> (the --msrv argument value)
  OUTPUT: boolean
  
  IF input is None THEN RETURN false (no bug - using default)
  END IF
  
  LET version = input.unwrap()
  LET trimmed = version.trim()
  
  -- Invalid if it starts with 'v' or 'latest'
  IF trimmed starts with 'v' OR trimmed == 'latest' THEN RETURN true
  END IF
  
  -- Split on '.' and validate each component
  LET parts = trimmed.split('.')
  
  IF parts.length < 2 OR parts.length > 3 THEN RETURN true
  END IF
  
  FOR EACH part IN parts DO
    IF part is empty OR part contains non-digit characters THEN RETURN true
    END IF
  END FOR
  
  -- Valid format: major.minor or major.minor.patch
  RETURN false
END FUNCTION
```

### Examples

1. **Valid (should NOT trigger bug condition)**: `--msrv 1.84` → accepted, written to workflow
2. **Valid (should NOT trigger bug condition)**: `--msrv 1.84.0` → accepted, written to workflow
3. **Invalid (triggers bug condition)**: `--msrv latest` → currently accepted (WRONG), should be rejected
4. **Invalid (triggers bug condition)**: `--msrv 1.84.0.1` → currently accepted (WRONG), should be rejected
5. **Invalid (triggers bug condition)**: `--msrv v1.84` → currently accepted (WRONG), should be rejected
6. **Invalid (triggers bug condition)**: `--msrv 1` → currently accepted (WRONG), should be rejected
7. **Valid (no bug - default)**: no `--msrv` flag → uses default `1.84`, written to workflow
8. **Edge case - Invalid (triggers bug condition)**: `--msrv 1.84.` → trailing dot, should be rejected
9. **Edge case - Invalid (triggers bug condition)**: `--msrv .1.84` → leading dot, should be rejected
10. **Edge case - Invalid (triggers bug condition)**: `--msrv 1..84` → double dot, should be rejected

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- When `--msrv 1.84` (major.minor format) is provided, it must continue to be accepted and written to the workflow
- When `--msrv 1.84.0` (major.minor.patch format) is provided, it must continue to be accepted and written to the workflow
- When no `--msrv` is provided, the default value of `1.84` must continue to be used
- All other `ci-init` flags (e.g., `--provider`, `--matrix`, `--deploy`, etc.) must continue to work exactly as before
- File writing behavior for all valid workflows must be unchanged

**Scope:**
All inputs where `--msrv` is either not provided or contains a valid version string should be completely unaffected by this fix. This includes:
- Valid major.minor versions
- Valid major.minor.patch versions
- Default MSRV when the flag is omitted
- All other feature flags and options

## Hypothesized Root Cause

Based on the bug description and code analysis, the root cause is straightforward:

1. **No Validation on --msrv Input**: The `GenerateOptions::msrv` field is populated directly from the command-line argument without any validation. The clap argument definition has no constraints, allowing any string to be accepted.

2. **Direct Variable Substitution**: The `generate()` function inserts the `--msrv` value directly into the template variables without checking its format. This value is then used to render template files via `render_str()` and written to disk.

3. **No Early Rejection Point**: The current code has no early validation gate. Even if the version string is invalid, file writing proceeds normally, making the error only apparent when the CI runs later.

## Correctness Properties

Property 1: Bug Condition - Invalid MSRV Values Are Rejected

_For any_ command invocation where `--msrv` is provided with an invalid version format (not matching `major.minor` or `major.minor.patch`), the `generate()` function SHALL immediately return a `ForgeError::InvalidArgument` with a clear error message indicating the required format, BEFORE any files are written to disk.

**Validates: Requirements 2.1, 2.2**

Property 2: Preservation - Valid Versions and Default Behavior

_For any_ command invocation where `--msrv` is either omitted (using the default) or provided with a valid version format, the `generate()` function SHALL proceed exactly as it currently does, accepting the version and writing all workflow files correctly, preserving all existing behavior.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4**

## Fix Implementation

### Changes Required

Assuming our root cause analysis is correct, the fix requires:

**File**: `crates/ci-presets/src/lib.rs`

**New Function**: `validate_msrv(version: &str) -> Result<()>`

1. **Add MSRV Validation Function**: Create a new function `validate_msrv()` that:
   - Takes the MSRV string as input
   - Returns `Result<()>` using the soroban-forge error type
   - Validates the format matches `major.minor` or `major.minor.patch`
   - Returns `ForgeError::InvalidArgument` with a clear error message if validation fails
   - Uses regex or manual parsing to check the format

2. **Early Validation Gate in generate()**: In the `generate()` function:
   - Add validation call BEFORE any `std::fs::create_dir_all()` or file writing operations
   - If `opts.msrv` is `Some(ref version)`, call `validate_msrv(version)?`
   - This ensures validation fails early, before directory creation or any side effects
   - Must occur before line ~119 where `std::fs::create_dir_all(&out_dir)` is called

3. **Validation Logic**: The validation function should:
   - Trim whitespace from the input
   - Reject strings starting with 'v' (e.g., `v1.84`)
   - Reject exact strings like `latest`, `stable`, `nightly`
   - Split on '.' and validate that:
     - There are exactly 2 or 3 components
     - Each component is a non-empty string of digits only
     - No leading zeros on the first digit of each component (optional - depends on preference)
   - Reject strings with adjacent dots, leading/trailing dots

4. **Error Message**: The error message should clearly state:
   - What was provided
   - What the valid format is (major.minor or major.minor.patch)
   - Example of valid values

5. **Implementation Language**: Rust code using either:
   - Manual parsing with string split/iteration
   - Regex matching against a pattern like `^\d+\.\d+(?:\.\d+)?$`

### Example Implementation Approach

```rust
/// Validates that an MSRV string matches the required format:
/// - major.minor (e.g., "1.84")
/// - major.minor.patch (e.g., "1.84.0")
///
/// # Errors
/// Returns InvalidArgument if the format is invalid.
fn validate_msrv(version: &str) -> Result<()> {
    let trimmed = version.trim();
    
    // Reject common invalid patterns
    if trimmed.starts_with('v') || trimmed == "latest" || trimmed == "stable" || trimmed == "nightly" {
        return Err(ForgeError::InvalidArgument(
            format!("invalid MSRV format '{}': must be major.minor or major.minor.patch (e.g., 1.84 or 1.84.0)", trimmed)
        ));
    }
    
    let parts: Vec<&str> = trimmed.split('.').collect();
    
    // Must have exactly 2 or 3 parts
    if parts.len() < 2 || parts.len() > 3 {
        return Err(ForgeError::InvalidArgument(
            format!("invalid MSRV format '{}': must be major.minor or major.minor.patch (e.g., 1.84 or 1.84.0)", trimmed)
        ));
    }
    
    // Each part must be a non-empty string of digits
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
            return Err(ForgeError::InvalidArgument(
                format!("invalid MSRV format '{}': each component must be a number (e.g., 1.84 or 1.84.0)", trimmed)
            ));
        }
    }
    
    Ok(())
}
```

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate the bug on unfixed code, then verify the fix works correctly and preserves existing behavior.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bug BEFORE implementing the fix. Run these tests on the UNFIXED code to see failures, confirming the bug exists.

**Test Plan**: Write tests that call `generate()` with various invalid `--msrv` values and assert that an `InvalidArgument` error is returned before any files are created.

**Test Cases**:
1. **Latest Keyword Test**: Simulate `--msrv latest` and verify it would currently be accepted (bug manifested on unfixed code)
2. **Versioning with 'v' Prefix Test**: Simulate `--msrv v1.84` and verify it would currently be accepted (bug manifested on unfixed code)
3. **Four-Part Version Test**: Simulate `--msrv 1.84.0.1` and verify it would currently be accepted (bug manifested on unfixed code)
4. **Single Component Test**: Simulate `--msrv 1` and verify it would currently be accepted (bug manifested on unfixed code)
5. **Trailing Dot Test**: Simulate `--msrv 1.84.` and verify it would currently be accepted (bug manifested on unfixed code)

**Expected Counterexamples**:
- On unfixed code: `generate()` accepts invalid versions and writes files
- After fix: `generate()` rejects invalid versions with `ForgeError::InvalidArgument` before any file writes

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds (invalid MSRV), the fixed function produces the expected behavior (rejection with error message).

**Pseudocode:**
```
FOR ALL input WHERE isBugCondition(input) DO
  result := generate_fixed(dir, provider, name, opts_with_invalid_msrv)
  ASSERT result is Err(ForgeError::InvalidArgument)
  ASSERT error message contains format description
  ASSERT no files were written to disk
END FOR
```

**Test Implementation**: 
- Test invalid MSRV values: `latest`, `v1.84`, `1.84.0.1`, `1`, `1.84.`, `.1.84`, `1..84`, `1.84.0.0`, `abc`, `1.a`, `1..`, `a.b.c`
- For each invalid value, assert:
  - Return value is `ForgeError::InvalidArgument`
  - Error message describes the valid format
  - No files created in the output directory
- Verify files are not created by checking directory state before and after

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold (valid MSRV or no MSRV), the fixed function produces the same result as the original function.

**Pseudocode:**
```
FOR ALL input WHERE NOT isBugCondition(input) DO
  result_original := generate_original(dir, provider, name, opts)
  result_fixed := generate_fixed(dir, provider, name, opts)
  ASSERT result_original = result_fixed
  ASSERT same files written in both cases
  ASSERT same file contents in both cases
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:
- It generates many valid test cases automatically
- It catches edge cases in version number parsing that manual tests might miss
- It provides strong guarantees that behavior is unchanged for all valid inputs

**Test Plan**: Run preservation tests for:
1. All valid major.minor combinations (small numbers: 1.0 through 2.99, and boundary cases)
2. All valid major.minor.patch combinations
3. Default MSRV (omitted flag)
4. Various providers (github, gitlab, bitbucket, etc.)
5. Various feature combinations (matrix, deploy, security_scan, etc.)

**Test Cases**:
1. **Valid Major.Minor Test**: `--msrv 1.84` produces same result before and after fix
2. **Valid Major.Minor.Patch Test**: `--msrv 1.84.0` produces same result before and after fix
3. **Default MSRV Test**: No `--msrv` flag uses default `1.84` exactly as before
4. **Large Version Numbers Test**: `--msrv 99.999.999` is accepted (same as before)
5. **Zero Components Test**: `--msrv 0.0.0` is accepted (same as before)
6. **Matrix Flag Test**: `--matrix --msrv 1.84` produces correct matrix workflow
7. **Other Flags Test**: `--deploy --msrv 1.84` produces both deploy and base workflows with correct MSRV
8. **Provider Variations Test**: Same MSRV works across all providers (github, gitlab, etc.)

### Unit Tests

- Create `validate_msrv()` function with comprehensive unit test coverage
- Test each invalid pattern individually (v-prefix, latest, wrong component count, non-numeric, etc.)
- Test boundary cases (empty string, whitespace only, zero values, large numbers)
- Test edge cases (leading/trailing dots, consecutive dots)
- Test valid patterns individually (1.0, 1.84, 1.84.0, etc.)
- Assert correct error messages for each invalid case
- Verify no file system side effects during validation

### Property-Based Tests

- Generate random valid major.minor and major.minor.patch versions
- For each generated valid version, verify it passes `validate_msrv()`
- For each generated valid version, verify `generate()` writes the same files before and after fix
- Use property-based testing library (quickcheck or similar) to generate many combinations
- Generate random combinations of flags and MSRV values, verify no regressions
- Test that all generated files contain the correct MSRV value in the right places

### Integration Tests

- End-to-end test of the full `ci-init` command with invalid MSRV
- End-to-end test of the full `ci-init` command with valid MSRV
- Verify that `soroban-forge ci-init --provider github --matrix --msrv invalid` exits with code 1 (UserError)
- Verify that error message is printed to stderr
- Verify that no workflow files are created when validation fails
- Test across different operating systems (Windows, macOS, Linux) if applicable
- Verify that valid workflows pass linting/validation tools
