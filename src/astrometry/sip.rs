use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

use crate::astrometry::StarMatch;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SipDistortion {
    pub order: usize,
    pub a: [[f64; 5]; 5],
    pub b: [[f64; 5]; 5],
    #[serde(default)]
    pub a_err: [[f64; 5]; 5],
    #[serde(default)]
    pub b_err: [[f64; 5]; 5],
    #[serde(default)]
    pub a_boot_err: [[f64; 5]; 5],
    #[serde(default)]
    pub b_boot_err: [[f64; 5]; 5],
    #[serde(default)]
    pub fit_rmse_pixels: f64,
}

impl Default for SipDistortion {
    fn default() -> Self {
        Self {
            order: 2,
            a: [[0.0; 5]; 5],
            b: [[0.0; 5]; 5],
            a_err: [[0.0; 5]; 5],
            b_err: [[0.0; 5]; 5],
            a_boot_err: [[0.0; 5]; 5],
            b_boot_err: [[0.0; 5]; 5],
            fit_rmse_pixels: 0.0,
        }
    }
}

impl SipDistortion {
    pub fn new(order: usize) -> Self {
        Self {
            order: order.clamp(2, 4),
            a: [[0.0; 5]; 5],
            b: [[0.0; 5]; 5],
            a_err: [[0.0; 5]; 5],
            b_err: [[0.0; 5]; 5],
            a_boot_err: [[0.0; 5]; 5],
            b_boot_err: [[0.0; 5]; 5],
            fit_rmse_pixels: 0.0,
        }
    }

    /// Apply forward SIP transform: un-distorted relative pixel (u, v) -> distorted pixel (u', v')
    pub fn apply_forward(&self, u: f64, v: f64) -> (f64, f64) {
        let mut du = 0.0;
        let mut dv = 0.0;

        for p in 0..=self.order {
            for q in 0..=self.order {
                let deg = p + q;
                if deg >= 2 && deg <= self.order {
                    let term = u.powi(p as i32) * v.powi(q as i32);
                    du += self.a[p][q] * term;
                    dv += self.b[p][q] * term;
                }
            }
        }

        (u + du, v + dv)
    }

    /// Apply inverse SIP transform: distorted relative pixel (u', v') -> un-distorted pixel (u, v)
    pub fn apply_inverse(&self, up: f64, vp: f64) -> (f64, f64) {
        let mut u = up;
        let mut v = vp;

        for _ in 0..30 {
            let (test_up, test_vp) = self.apply_forward(u, v);
            let err_u = test_up - up;
            let err_v = test_vp - vp;

            u -= err_u;
            v -= err_v;

            if err_u.abs() < 1e-10 && err_v.abs() < 1e-10 {
                break;
            }
        }

        (u, v)
    }

    /// Fit SIP coefficients from matched stars and field center (cx, cy)
    pub fn fit_from_residuals(matches: &[StarMatch], cx: f64, cy: f64, order: usize) -> Self {
        let order = order.clamp(2, 4);
        let mut sip = SipDistortion::new(order);

        let mut terms = Vec::new();
        for p in 0..=order {
            for q in 0..=order {
                let deg = p + q;
                if deg >= 2 && deg <= order {
                    terms.push((p, q));
                }
            }
        }

        let num_terms = terms.len();
        let num_matches = matches.len();

        if num_matches < num_terms {
            return sip;
        }

        let mut m_mat = DMatrix::<f64>::zeros(num_matches, num_terms);
        let mut du_vec = DVector::<f64>::zeros(num_matches);
        let mut dv_vec = DVector::<f64>::zeros(num_matches);

        for (i, m_star) in matches.iter().enumerate() {
            let u_det = m_star.pixel_x - cx;
            let v_det = m_star.pixel_y - cy;

            // Catalog coordinate approximate relative position
            let u_cat = u_det;
            let v_cat = v_det;

            // Residual pixel offset (directional, not scalar magnitude)
            let du = m_star.dx_pixels.clamp(-50.0, 50.0);
            let dv = m_star.dy_pixels.clamp(-50.0, 50.0);

            for (j, &(p, q)) in terms.iter().enumerate() {
                m_mat[(i, j)] = u_cat.powi(p as i32) * v_cat.powi(q as i32);
            }
            du_vec[i] = du;
            dv_vec[i] = dv;
        }

        let m_t = m_mat.transpose();
        let mtm = &m_t * &m_mat;
        let mt_du = &m_t * du_vec;
        let mt_dv = &m_t * dv_vec;

        if let Some(a_sol) = mtm.clone().qr().solve(&mt_du) {
            for (j, &(p, q)) in terms.iter().enumerate() {
                sip.a[p][q] = a_sol[j];
            }
        }

        if let Some(b_sol) = mtm.clone().qr().solve(&mt_dv) {
            for (j, &(p, q)) in terms.iter().enumerate() {
                sip.b[p][q] = b_sol[j];
            }
        }

        let mut sq_err_u = 0.0;
        let mut sq_err_v = 0.0;
        for m_star in matches {
            let u_det = m_star.pixel_x - cx;
            let v_det = m_star.pixel_y - cy;
            let (pred_u, pred_v) = sip.apply_forward(u_det, v_det);
            let target_u = u_det + m_star.dx_pixels;
            let target_v = v_det + m_star.dy_pixels;
            let du = pred_u - target_u;
            let dv = pred_v - target_v;
            sq_err_u += du * du;
            sq_err_v += dv * dv;
        }

        let dof = (num_matches as f64 - num_terms as f64).max(1.0);
        let var_u = sq_err_u / dof;
        let var_v = sq_err_v / dof;

        if let Some(mtm_inv) = mtm.try_inverse() {
            for (j, &(p, q)) in terms.iter().enumerate() {
                let diag = mtm_inv[(j, j)];
                if diag > 0.0 {
                    sip.a_err[p][q] = (var_u * diag).sqrt();
                    sip.b_err[p][q] = (var_v * diag).sqrt();
                }
            }
        }

        sip.fit_rmse_pixels = ((sq_err_u + sq_err_v) / matches.len() as f64).sqrt();

        sip
    }

    /// Fit SIP directly from pairs of (u_undistorted, v_undistorted) and (u_distorted, v_distorted)
    #[allow(clippy::type_complexity)]
    pub fn fit_from_point_pairs(pairs: &[((f64, f64), (f64, f64))], order: usize) -> Self {
        let order = order.clamp(2, 4);
        let mut sip = SipDistortion::new(order);

        let mut terms = Vec::new();
        for p in 0..=order {
            for q in 0..=order {
                let deg = p + q;
                if deg >= 2 && deg <= order {
                    terms.push((p, q));
                }
            }
        }

        let num_terms = terms.len();
        let num_pairs = pairs.len();

        if num_pairs < num_terms {
            return sip;
        }

        let mut m_mat = DMatrix::<f64>::zeros(num_pairs, num_terms);
        let mut du_vec = DVector::<f64>::zeros(num_pairs);
        let mut dv_vec = DVector::<f64>::zeros(num_pairs);

        for (i, &((u, v), (up, vp))) in pairs.iter().enumerate() {
            let du = up - u;
            let dv = vp - v;

            for (j, &(p, q)) in terms.iter().enumerate() {
                m_mat[(i, j)] = u.powi(p as i32) * v.powi(q as i32);
            }
            du_vec[i] = du;
            dv_vec[i] = dv;
        }

        let m_t = m_mat.transpose();
        let mtm = &m_t * &m_mat;
        let mt_du = &m_t * du_vec;
        let mt_dv = &m_t * dv_vec;

        if let Some(a_sol) = mtm.clone().qr().solve(&mt_du) {
            for (j, &(p, q)) in terms.iter().enumerate() {
                sip.a[p][q] = a_sol[j];
            }
        }

        if let Some(b_sol) = mtm.clone().qr().solve(&mt_dv) {
            for (j, &(p, q)) in terms.iter().enumerate() {
                sip.b[p][q] = b_sol[j];
            }
        }

        let mut sq_err_u = 0.0;
        let mut sq_err_v = 0.0;
        for &((u, v), (up, vp)) in pairs {
            let (pred_up, pred_vp) = sip.apply_forward(u, v);
            let du = pred_up - up;
            let dv = pred_vp - vp;
            sq_err_u += du * du;
            sq_err_v += dv * dv;
        }

        let dof = (num_pairs as f64 - num_terms as f64).max(1.0);
        let var_u = sq_err_u / dof;
        let var_v = sq_err_v / dof;

        if let Some(mtm_inv) = mtm.try_inverse() {
            for (j, &(p, q)) in terms.iter().enumerate() {
                let diag = mtm_inv[(j, j)];
                if diag > 0.0 {
                    sip.a_err[p][q] = (var_u * diag).sqrt();
                    sip.b_err[p][q] = (var_v * diag).sqrt();
                }
            }
        }

        sip.fit_rmse_pixels = ((sq_err_u + sq_err_v) / pairs.len() as f64).sqrt();

        sip
    }

    /// Fit SIP and compute empirical bootstrap standard errors (num_boot iterations)
    pub fn fit_with_bootstrap(
        pairs: &[((f64, f64), (f64, f64))],
        order: usize,
        num_boot: usize,
    ) -> Self {
        let mut sip = Self::fit_from_point_pairs(pairs, order);
        let (a_boot, b_boot) = Self::compute_bootstrap_errors(pairs, order, num_boot);
        sip.a_boot_err = a_boot;
        sip.b_boot_err = b_boot;
        sip
    }

    /// Compute empirical bootstrap standard errors (B resamples with replacement)
    pub fn compute_bootstrap_errors(
        pairs: &[((f64, f64), (f64, f64))],
        order: usize,
        num_boot: usize,
    ) -> ([[f64; 5]; 5], [[f64; 5]; 5]) {
        let n = pairs.len();
        if n < 4 || num_boot == 0 {
            return ([[0.0; 5]; 5], [[0.0; 5]; 5]);
        }

        let mut rng = rand::thread_rng();
        let mut a_samples: Vec<Vec<f64>> = vec![Vec::with_capacity(num_boot); 25];
        let mut b_samples: Vec<Vec<f64>> = vec![Vec::with_capacity(num_boot); 25];

        for _ in 0..num_boot {
            let mut sample = Vec::with_capacity(n);
            for _ in 0..n {
                let idx = rand::Rng::gen_range(&mut rng, 0..n);
                sample.push(pairs[idx]);
            }
            let sip_b = Self::fit_from_point_pairs(&sample, order);
            for p in 0..=order {
                for q in 0..=order {
                    if p + q >= 2 && p + q <= order {
                        a_samples[p * 5 + q].push(sip_b.a[p][q]);
                        b_samples[p * 5 + q].push(sip_b.b[p][q]);
                    }
                }
            }
        }

        let mut a_boot_err = [[0.0; 5]; 5];
        let mut b_boot_err = [[0.0; 5]; 5];

        for p in 0..=order {
            for q in 0..=order {
                if p + q >= 2 && p + q <= order {
                    let idx = p * 5 + q;
                    let a_vals = &a_samples[idx];
                    let b_vals = &b_samples[idx];
                    if !a_vals.is_empty() {
                        let mean_a: f64 = a_vals.iter().sum::<f64>() / a_vals.len() as f64;
                        let var_a: f64 = a_vals
                            .iter()
                            .map(|v| (v - mean_a).powi(2))
                            .sum::<f64>()
                            / a_vals.len() as f64;
                        a_boot_err[p][q] = var_a.sqrt();
                    }
                    if !b_vals.is_empty() {
                        let mean_b: f64 = b_vals.iter().sum::<f64>() / b_vals.len() as f64;
                        let var_b: f64 = b_vals
                            .iter()
                            .map(|v| (v - mean_b).powi(2))
                            .sum::<f64>()
                            / b_vals.len() as f64;
                        b_boot_err[p][q] = var_b.sqrt();
                    }
                }
            }
        }

        (a_boot_err, b_boot_err)
    }

    /// Print individual parameter values and their fit errors cleanly
    pub fn print_fit_results(&self) {
        println!("  SIP Polynomial Order: {}", self.order);
        println!("  Overall Fit RMSE: {:.3} px", self.fit_rmse_pixels);
        println!("  Individual Parameter Fit Results & Errors:");
        for p in 0..=self.order {
            for q in 0..=self.order {
                let deg = p + q;
                if deg >= 2 && deg <= self.order {
                    if self.a_boot_err[p][q] > 0.0 {
                        println!(
                            "    A_{p}_{q}: {:+.2e} ± {:.2e} (boot_err: {:.2e})",
                            self.a[p][q], self.a_err[p][q], self.a_boot_err[p][q]
                        );
                        println!(
                            "    B_{p}_{q}: {:+.2e} ± {:.2e} (boot_err: {:.2e})",
                            self.b[p][q], self.b_err[p][q], self.b_boot_err[p][q]
                        );
                    } else {
                        println!(
                            "    A_{p}_{q}: {:+.2e} ± {:.2e}",
                            self.a[p][q], self.a_err[p][q]
                        );
                        println!(
                            "    B_{p}_{q}: {:+.2e} ± {:.2e}",
                            self.b[p][q], self.b_err[p][q]
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sip_forward_inverse() {
        let mut sip = SipDistortion::new(3);
        sip.a[2][0] = 1e-5;
        sip.a[0][2] = 2e-5;
        sip.b[1][1] = 1.5e-5;
        sip.b[3][0] = 5e-9;

        let u_orig = 120.0;
        let v_orig = -85.0;

        let (up, vp) = sip.apply_forward(u_orig, v_orig);
        let (u_rec, v_rec) = sip.apply_inverse(up, vp);

        let err = (u_rec - u_orig).hypot(v_rec - v_orig);
        assert!(
            err < 0.01,
            "Round-trip SIP transform error should be < 0.01 px, got {err}"
        );
    }

    #[test]
    fn test_sip_fit() {
        // Construct synthetic cubic distortion: du = k1 * u * (u^2 + v^2)
        // For p=3,q=0: A_3_0 = k1; for p=1,q=2: A_1_2 = k1
        let k1 = 1e-7;
        let mut pairs = Vec::new();

        for x in (-400..=400).step_by(50) {
            for y in (-400..=400).step_by(50) {
                let u = x as f64;
                let v = y as f64;
                let r2 = u * u + v * v;
                let up = u + k1 * u * r2;
                let vp = v + k1 * v * r2;
                pairs.push(((u, v), (up, vp)));
            }
        }

        let sip = SipDistortion::fit_from_point_pairs(&pairs, 3);
        let a_3_0 = sip.a[3][0];
        let a_1_2 = sip.a[1][2];

        assert!(
            (a_3_0 - k1).abs() / k1 < 0.10,
            "A_3_0 should recover k1 within 10%, got {a_3_0}"
        );
        assert!(
            (a_1_2 - k1).abs() / k1 < 0.10,
            "A_1_2 should recover k1 within 10%, got {a_1_2}"
        );
        assert!(
            sip.fit_rmse_pixels < 0.1,
            "SIP fit RMSE should be < 0.1 px for synthetic model, got {}",
            sip.fit_rmse_pixels
        );
    }

    #[test]
    fn test_sip_fit_errors() {
        let mut pairs = Vec::new();
        for x in (-300..=300).step_by(40) {
            for y in (-300..=300).step_by(40) {
                let u = x as f64;
                let v = y as f64;
                let up = u + 1e-5 * u * u;
                let vp = v + 2e-5 * v * v;
                pairs.push(((u, v), (up, vp)));
            }
        }
        let sip = SipDistortion::fit_from_point_pairs(&pairs, 2);
        assert!(sip.a_err[2][0] >= 0.0);
        assert!(sip.b_err[0][2] >= 0.0);
        assert!(!sip.a_err[2][0].is_nan());
        assert!(!sip.b_err[0][2].is_nan());
    }
}
