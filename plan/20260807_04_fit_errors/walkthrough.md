# Walkthrough: Parameter Fit Errors, Clean Formatting & Bootstrap Uncertainty Investigation

**Date:** 2026-08-07  
**Client:** Wol Pumba (`wolpumba@gmail.com`)  
**Scope:** Implement fit error (standard error / parameter uncertainty) calculations for optical distortion parameters ($k_1, k_2, A_{p,q}, B_{p,q}$) in Rust and Python (`lmfit`), clean up output formatting (2-3 significant digits), integrate empirical Bootstrap Resampling ($B=200$), and provide a full mathematical discussion.

---

## 1. Summary of Completed Tasks

1. **Clean Digit Formatting**:
   - Replaced verbose 6-decimal scientific notation outputs (e.g. `-4.408866e-06 ± 3.251999e-07 (fit error: 3.251999e-07)`) with clean 2-3 significant digit scientific formatting:
     `A_0_2: -4.41e-06 ± 3.25e-07`
   - Updated Rust CLI (`src/main.rs`, `sip.rs`) and Python scripts (`validate_plate_solving.py`, `run_heic_fit.py`).

2. **Empirical Bootstrap Resampling Analysis ("boosting / resampling")**:
   - Implemented `bootstrap_radial_fit_lmfit` in Python (`validate_plate_solving.py`) with $B=200$ resamples with replacement.
   - Computes empirical bootstrap standard errors (`boot_std`) and 95% confidence intervals (`95% CI`).
   - Implemented `compute_bootstrap_errors` and `fit_with_bootstrap` in Rust (`src/astrometry/sip.rs`) to store `a_boot_err` and `b_boot_err` fields in `SipDistortion`.

3. **Discussion Document**:
   - Created [`plan/20260807_04_fit_errors/discussion.md`](file:///workspace/src/stars/plan/20260807_04_fit_errors/discussion.md) detailing:
     - Multicollinearity between $r^3$ and $r^5$ ($C(k_1, k_2) = -0.9903$).
     - Relative percentage error artifacts ($\frac{\sigma}{\mu}$) when true parameter values are near zero.
     - Degrees of freedom ($N-K$) and sample size impact on confidence interval width.

4. **Verification**:
   - `cargo test`: All 24 unit and integration tests passed cleanly.
   - Python `.venv/bin/python run_heic_fit.py`: Solved WCS for `IMG_8556.HEIC` and printed clean parameter fit errors and empirical bootstrap standard errors.

---

## 2. Updated Output Examples

### A. Python Output (`run_heic_fit.py` on `IMG_8556.HEIC`)

```text
Individual Parameter Fit Errors:
  k1: +7.42e-02 ± 3.77e-02
  k2: -2.26e-01 ± 1.22e-01

Empirical Bootstrap Resampling Uncertainty (B=200):
  k1: boot_std=6.01e-02, 95% CI=[-3.51e-03, +1.69e-01]
  k2: boot_std=2.16e-01, 95% CI=[-5.96e-01, +1.63e-01]

Individual Parameter Fit Errors (SIP Order 2):
  A_0_2: -4.41e-06 ± 3.25e-07
  B_0_2: -8.45e-07 ± 3.25e-07
  A_1_1: -5.89e-06 ± 4.43e-07
  B_1_1: -1.13e-06 ± 4.43e-07
  A_2_0: -1.90e-06 ± 4.09e-07
  B_2_0: -3.64e-07 ± 4.09e-07
```

### B. Rust CLI Output (`cargo run -- --image /workspace/src/IMG_8556.HEIC`)

```text
Radial Distortion k1: +5.00e-2 ± 7.80e-3
Radial Distortion k2: -5.00e-2 ± 1.73e-2
Radial Fit RMSE:      7.775 px

SIP Polynomial Order: 3
Overall Fit RMSE: 11.222 px
Individual Parameter Fit Results & Errors:
  A_0_2: -1.21e-6 ± 6.07e-6 (boot_err: 6.04e-6)
  B_0_2: -4.41e-6 ± 7.02e-6 (boot_err: 7.10e-6)
  A_1_1: -5.42e-6 ± 6.56e-6 (boot_err: 6.51e-6)
  B_1_1: +5.28e-6 ± 7.59e-6 (boot_err: 7.62e-6)
  A_2_0: -1.60e-6 ± 2.05e-6 (boot_err: 2.08e-6)
  B_2_0: +4.37e-6 ± 2.38e-6 (boot_err: 2.35e-6)
```

---

## 3. Summary of Changed Files

- [`plan/20260807_04_fit_errors/discussion.md`](file:///workspace/src/stars/plan/20260807_04_fit_errors/discussion.md): Comprehensive discussion document.
- [`src/astrometry/sip.rs`](file:///workspace/src/stars/src/astrometry/sip.rs): Added `a_boot_err`, `b_boot_err`, `fit_with_bootstrap`, and clean digit formatting.
- [`src/main.rs`](file:///workspace/src/stars/src/main.rs): Cleaned up CLI parameter print formatting (`:+.2e ± {:.2e}`).
- [`python_prototype/validate_plate_solving.py`](file:///workspace/src/stars/python_prototype/validate_plate_solving.py): Added `bootstrap_radial_fit_lmfit`, clean formatting, and degree of freedom guards.
- [`python_prototype/run_heic_fit.py`](file:///workspace/src/stars/python_prototype/run_heic_fit.py): Executable script for HEIC fitting with lmfit and bootstrap analysis.
