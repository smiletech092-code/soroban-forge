# Implementation Tasks - MSRV Validation Bugfix

## Phase 3: Create Implementation Tasks

### Objective
Implement the MSRV validation feature using the bug condition methodology, ensuring:
1. Bug Condition tests explore and demonstrate the bug on unfixed code
2. Preservation tests verify non-buggy behavior is unchanged
3. Implementation applies the fix with clear understanding
4. All tests validate the fix works without regressions

---

## Task List

- [ ] 1. Write bug condition exploration test
  - **Property 1: Bug Condition** - Invalid MSRV Values Are Currently Accepted
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **DO NOT attempt to fix the test or the code when it fails**
  - **GOAL**: Surface counterexamples that demonstrate the bug exists on unfixed code
  - **Scoped PBT Approach**: Scope to concrete failing cases: invalid versions like `latest`, `v1.84`, `1.84.0.1`, `1`, `1.84.`, `.1.84`, `1..84`
  - **Test Implementation Details from Bug Condition (design 1.1-1.2)**:
    - Test that `generate()` accepts `--msrv latest` without error on unfixed code
    - Test that `generate()` accepts `--msrv v1.84` without error on unfixed code
    - Test that `generate()` accepts `--msrv 1.84.0.1` without error on unfixed code
    - Test that `generate()` accepts `--msrv 1` without error on unfixed code
    - Test that `generate()` accepts `--msrv 1.84.` without error on unfixed code
    - Test that files ARE written with these invalid versions on unfixed code
  - **Expected Behavior (design Property 1)**: After fix, all these should return `ForgeError::InvalidArgument` BEFORE any files are written
  - Run test on UNFIXED code
  - **EXPECTED OUTCOME**: Test FAILS (this confirms the bug exists on unfixed code)
  - Document counterexamples found (e.g., "`generate()` accepts `latest` without validation on unfixed code")
  - Mark task complete when test is written, run on unfixed code, and failure is documented
  - _Requirements: 1.1, 1.2_

- [ ] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** - Valid MSRV Values And Default Behavior Are Unchanged
  - **CRITICAL**: Follow observation-first methodology
  - **IMPORTANT**: Write tests BEFORE implementing the fix
  - **GOAL**: Capture and preserve existing behavior for non-buggy inputs
  - Observe: `generate()` accepts `--msrv 1.84` on unfixed code and writes files
  - Observe: `generate()` accepts `--msrv 1.84.0` on unfixed code and writes files
  - Observe: `generate()` uses default `1.84` when no `--msrv` provided on unfixed code
  - Observe: Generated files contain the correct MSRV value in workflow templates
  - **Test Implementation Details from Preservation Requirements (design section)**:
    - Property-based test: For all valid major.minor versions (e.g., 1.0 through 99.999), `generate()` accepts them and writes files
    - Property-based test: For all valid major.minor.patch versions (e.g., 1.0.0 through 99.999.999), `generate()` accepts them and writes files
    - Unit test: `--msrv 1.84` produces identical output before and after fix
    - Unit test: `--msrv 1.84.0` produces identical output before and after fix
    - Unit test: No `--msrv` flag uses default `1.84` exactly as before
    - Unit test: Various feature combinations (matrix, deploy, coverage, etc.) work with valid MSRV
    - Integration test: Different providers (github, gitlab, etc.) accept same valid MSRV values
    - Verify files contain correct MSRV value in the right template locations
  - Run tests on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run on unfixed code, and all pass
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 3. Fix for MSRV validation

  - [ ] 3.1 Implement validate_msrv() function in crates/ci-presets/src/lib.rs
    - Create function signature: `fn validate_msrv(version: &str) -> Result<()>`
    - Validate format matches `major.minor` or `major.minor.patch` pattern
    - Reject strings starting with 'v' (e.g., `v1.84`)
    - Reject exact keywords: `latest`, `stable`, `nightly`
    - Reject versions with wrong component count (not 2 or 3 parts)
    - Reject versions with non-numeric components
    - Reject versions with leading dots, trailing dots, or consecutive dots
    - Return `ForgeError::InvalidArgument` with clear error message: "invalid MSRV format 'X': must be major.minor or major.minor.patch (e.g., 1.84 or 1.84.0)"
    - Add unit tests for validate_msrv() with comprehensive coverage:
      - Valid cases: `1.84`, `1.84.0`, `0.0.0`, `99.999.999`
      - Invalid cases: `latest`, `v1.84`, `1.84.0.1`, `1`, `1.84.`, `.1.84`, `1..84`, ``, `a.b.c`, `1.a`
    - _Bug_Condition: isBugCondition(input) where input does not match major.minor or major.minor.patch pattern_
    - _Expected_Behavior: Return error for invalid format before any side effects (design Property 1)_
    - _Requirements: 2.1, 2.2_

  - [ ] 3.2 Integrate validation into generate() function
    - Add validation call early in `generate()` function, BEFORE `std::fs::create_dir_all()`
    - Call `validate_msrv(version)?` if `opts.msrv` is `Some(ref version)`
    - Validation must occur before line ~119 where directory creation happens
    - Ensure error is returned immediately without any file system side effects
    - Add inline comment explaining validation placement
    - _Bug_Condition: isBugCondition from design - invalid MSRV format_
    - _Expected_Behavior: Reject invalid MSRV with error before directory creation (design Property 1)_
    - _Preservation: Valid MSRV values and default continue to work unchanged (design Property 2)_
    - _Requirements: 2.1, 2.2, 3.1, 3.2, 3.3, 3.4_

  - [ ] 3.3 Verify bug condition exploration test from task 1 now passes
    - **Property 1: Expected Behavior** - Invalid MSRV Values Are Now Rejected
    - **CRITICAL**: Re-run the SAME test from task 1 - do NOT write a new test
    - The test from task 1 encodes the expected behavior
    - When this test passes, it confirms the expected behavior is satisfied
    - Run the bug condition exploration test from step 1 with the FIXED code
    - **EXPECTED OUTCOME**: Test PASSES (confirms bug is fixed)
    - All invalid MSRV values (latest, v1.84, 1.84.0.1, 1, etc.) now return `ForgeError::InvalidArgument`
    - All invalid version attempts produce files NOT created on disk
    - Error messages clearly indicate the required format
    - _Requirements: 2.1, 2.2, Expected Behavior Properties from design_

  - [ ] 3.4 Verify preservation tests from task 2 still pass
    - **Property 2: Preservation** - Valid MSRV And Default Behavior Unchanged
    - **CRITICAL**: Re-run the SAME tests from task 2 - do NOT write new tests
    - Run preservation property tests from step 2 with the FIXED code
    - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions)
    - Valid major.minor versions still accepted and written correctly
    - Valid major.minor.patch versions still accepted and written correctly
    - Default MSRV still used when flag omitted
    - Generated files still contain correct MSRV values in templates
    - All feature combinations (matrix, deploy, coverage, etc.) still work
    - All providers still work with same MSRV values
    - Confirm all tests still pass after fix (no regressions)
    - _Requirements: 3.1, 3.2, 3.3, 3.4, Preservation Requirements from design_

- [ ] 4. Checkpoint - Ensure all tests pass
  - Run full test suite for crates/ci-presets: `cargo test --package ci-presets`
  - Verify bug condition exploration test PASSES (indicates fix works)
  - Verify preservation tests PASS (indicates no regressions)
  - Verify unit tests for validate_msrv() PASS
  - Verify integration tests for full ci-init command PASS
  - Check test coverage: All error paths and valid cases covered
  - Document any edge cases discovered during testing
  - Ensure all tests pass, ask the user if questions arise

---

## Testing Notes

### Bug Condition Exploration (Task 1)
- **What to test on unfixed code**: Invalid MSRV values are accepted and files are written
- **What to test on fixed code**: Invalid MSRV values are rejected with `ForgeError::InvalidArgument` before files are written
- **Counterexamples to document**: 
  - `generate()` with `latest` writes files on unfixed code
  - `generate()` with `v1.84` writes files on unfixed code
  - `generate()` with `1.84.0.1` writes files on unfixed code
  - etc.

### Preservation Testing (Task 2)
- **What to test on unfixed code**: Valid MSRV values work correctly
- **What to test on fixed code**: Same valid MSRV values produce identical output
- **Property-based approach**: Generate many random valid versions and verify all are accepted and produce correct files

### Implementation (Task 3)
- **Validation function**: Pure function with no side effects, easy to unit test in isolation
- **Integration point**: Early in `generate()` to fail fast before any file operations
- **Error messages**: Clear and actionable, guide user to correct format
