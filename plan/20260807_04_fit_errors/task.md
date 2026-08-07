# Task: Parameter Fit Errors and Uncertainties Tracking (Rust & Python lmfit)

## Goal
Implement individual parameter fit error (standard error / uncertainty) reporting for optical distortion and astrometric fitting models in both Rust and Python (using `lmfit`).

## Requirements
- [x] Consult DeepWiki MCP for `lmfit` library usage (`lmfit.Parameters`, `lmfit.minimize`, `lmfit.fit_report`, `param.stderr`).
- [x] Integrate `lmfit` in Python prototype (`python_prototype/validate_plate_solving.py`) to fit radial ($k_1, k_2$) and 2D SIP polynomial ($A_{p,q}, B_{p,q}$) distortion parameters, printing individual parameter fit errors (`param.stderr`).
- [x] Add Python unit tests in `python_prototype/test_lmfit_fitting.py` verifying `lmfit` parameter standard error output.
- [x] Compute least-squares parameter standard errors in Rust (`SipDistortion` and `AberrationReport`) using covariance matrices $(M^T M)^{-1} \cdot \sigma^2$.
- [x] Update Rust CLI (`src/main.rs`) and `SipDistortion` to print parameter values and standard errors.
- [x] Add Rust unit test `test_sip_fit_errors` in `src/astrometry/sip.rs`.
- [x] Verify all Rust (`cargo test`) and Python test suites pass cleanly.
