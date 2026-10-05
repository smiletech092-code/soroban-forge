# Bugfix Requirements Document

## Introduction

The `--msrv` flag in the `ci-init` command accepts any string value and writes it directly into the generated CI workflow's toolchain matrix without validation. This allows invalid Rust version strings (e.g., `latest`, `1.84.0.1`, or `v1.84`) to be written into the workflow. These invalid values only fail confusingly when the CI runs, rather than failing locally with a clear error message. This bugfix adds validation to ensure only well-formed Rust versions (major.minor or major.minor.patch format) are accepted.

## Bug Analysis

### Current Behavior (Defect)

1.1 WHEN the user passes `--msrv` with an invalid version format (e.g., `latest`, `1.84.0.1`, `v1.84`, or `1`) THEN the system accepts the value and writes it into the generated workflow files without any validation

1.2 WHEN the user passes `--msrv` with an invalid format and the workflow is run in CI THEN the workflow fails with a confusing error from the CI system rather than a clear local error message

### Expected Behavior (Correct)

2.1 WHEN the user passes `--msrv` with an invalid version format THEN the system SHALL immediately reject the value with a clear error message indicating the required format (major.minor or major.minor.patch)

2.2 WHEN the user passes `--msrv` with an invalid format THEN the system SHALL NOT write any workflow files to disk

### Unchanged Behavior (Regression Prevention)

3.1 WHEN the user passes `--msrv` with a valid version format like `1.84` (major.minor) THEN the system SHALL CONTINUE TO accept the value and write it into the generated workflow files

3.2 WHEN the user passes `--msrv` with a valid version format like `1.84.0` (major.minor.patch) THEN the system SHALL CONTINUE TO accept the value and write it into the generated workflow files

3.3 WHEN the user does not pass `--msrv` (using the default) THEN the system SHALL CONTINUE TO use the default MSRV value of `1.84`

3.4 WHEN the user passes `--msrv` with a valid format for non-matrix workflows THEN the system SHALL CONTINUE TO write all other workflow files correctly
