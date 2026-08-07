# Comprehensive Investigation & Discussion of Fit Errors and Parameter Uncertainties

**Author:** Wol Pumba (`wolpumba@gmail.com`)  
**Date:** 2026-08-07  
**Scope:** Deep-dive analysis comparing asymptotic linear covariance standard errors vs empirical bootstrap resampling ($B=200$) for optical distortion models ($k_1, k_2, A_{p,q}, B_{p,q}$).

---

## 1. Executive Summary

When fitting optical distortion models (such as radial distortion polynomials $\Delta r = k_1 r^3 + k_2 r^5$ or 2D SIP polynomial matrices $A_{p,q}, B_{p,q}$), raw reported relative errors (e.g. $+1367\%$ or $\pm 0.12$) can initially appear surprisingly large.

Our investigation demonstrates that this behavior is driven by **three distinct mathematical mechanisms**:

1. **Ill-Conditioning & Multicollinearity ($C(k_1, k_2) = -0.9903$)**: Basis functions $r^3$ and $r^5$ are nearly collinear over $r \in [0, 1]$.
2. **Relative Percentage Artifacts on Zero-Valued Parameters**: Parameters with ground-truth values near zero (e.g. $A_{0,2} \approx 0$) yield high percentage errors ($\frac{\sigma}{\mu}$) despite absolute errors being tiny ($\sim 10^{-7}$).
3. **Sample Size & Degree-of-Freedom Constraints**: When matching few catalog stars ($N \sim 4\text{--}6$), confidence intervals naturally expand.

---

## 2. Asymptotic Covariance vs Bootstrap Resampling Comparison

### A. Radial Distortion Model ($k_1, k_2$) on `IMG_8556.HEIC`

| Metric / Method | $k_1$ Parameter | $k_2$ Parameter |
| :--- | :--- | :--- |
| **Best-Fit Estimate ($\hat{\beta}$)** | $+7.42 \times 10^{-2}$ | $-2.26 \times 10^{-1}$ |
| **Asymptotic Covariance Standard Error (`stderr`)** | $\pm 3.77 \times 10^{-2}$ | $\pm 1.22 \times 10^{-1}$ |
| **Empirical Bootstrap Standard Error (`boot_std`, $B=200$)** | $6.01 \times 10^{-2}$ | $2.16 \times 10^{-1}$ |
| **Empirical 95% Confidence Interval** | $[-3.51 \times 10^{-3}, +1.69 \times 10^{-1}]$ | $[-5.96 \times 10^{-1}, +1.63 \times 10^{-1}]$ |

**Insight**:
- The asymptotic standard error underestimates uncertainty when sample size $N$ is small ($N=4$).
- Bootstrap resampling reveals that the empirical 95% confidence interval for $k_1$ spans $[-0.0035, +0.169]$, demonstrating the full parameter distribution without assuming rigid Gaussian linear independence.

---

### B. 2D SIP Polynomial Model ($A_{p,q}, B_{p,q}$)

| Parameter | Fitted Value | Asymptotic Fit Error (`stderr`) | Relative Error (%) |
| :--- | :--- | :--- | :--- |
| **$A_{0,2}$** | $-4.41 \times 10^{-6}$ | $\pm 3.25 \times 10^{-7}$ | $7.38\%$ |
| **$B_{0,2}$** | $-8.45 \times 10^{-7}$ | $\pm 3.25 \times 10^{-7}$ | $38.47\%$ |
| **$A_{1,1}$** | $-5.89 \times 10^{-6}$ | $\pm 4.43 \times 10^{-7}$ | $7.51\%$ |
| **$B_{1,1}$** | $-1.13 \times 10^{-6}$ | $\pm 4.43 \times 10^{-7}$ | $39.19\%$ |
| **$A_{2,0}$** | $-1.90 \times 10^{-6}$ | $\pm 4.09 \times 10^{-7}$ | $21.55\%$ |
| **$B_{2,0}$** | $-3.64 \times 10^{-7}$ | $\pm 4.09 \times 10^{-7}$ | $112.38\%$ |

**Key Findings**:
1. All absolute standard errors are consistently $\sim 3\text{--}4 \times 10^{-7}$ px.
2. The relative percentage error for $B_{2,0}$ ($112\%$) occurs simply because the estimated coefficient $-3.64 \times 10^{-7}$ is close to zero relative to the background noise floor.

---

## 3. Mathematical Mechanisms & Recommendations

### Mechanism 1: Multicollinearity Inflation
In linear regression $M \beta = y$, parameter standard errors are:
$$\text{SE}(\hat{\beta}_j) = \sqrt{\sigma^2 \cdot (M^T M)^{-1}_{jj}}$$
Because $r^3$ and $r^5$ have a high correlation coefficient ($C(k_1, k_2) = -0.9903$), $(M^T M)^{-1}_{jj}$ contains large diagonal elements. $k_1$ and $k_2$ trade off against each other in opposite directions while maintaining nearly identical predicted pixel displacements $\Delta r$.

### Recommendation for Small $N$
- When $N < 10$, fall back to a single-parameter model ($\Delta r = k_1 r^3$).
- When $N \ge 30$, fit full $k_1, k_2$ and 3rd-order SIP polynomials.

---

## 4. Conclusion
1. **Formatting**: Output formatting in Rust and Python has been updated to print clean scientific notation (`val ± err`) limited to 2 significant digits for errors.
2. **Empirical Validation**: Bootstrap resampling ($B=200$) is integrated to provide non-parametric 95% confidence intervals alongside asymptotic standard errors.
