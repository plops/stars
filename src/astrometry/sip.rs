use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

use crate::astrometry::StarMatch;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SipDistortion {
    pub order: usize,
    pub a: [[f64; 5]; 5],
    pub b: [[f64; 5]; 5],
}

impl Default for SipDistortion {
    fn default() -> Self {
        Self {
            order: 2,
            a: [[0.0; 5]; 5],
            b: [[0.0; 5]; 5],
        }
    }
}

impl SipDistortion {
    pub fn new(order: usize) -> Self {
        Self {
            order: order.clamp(2, 4),
            a: [[0.0; 5]; 5],
            b: [[0.0; 5]; 5],
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

        if let Some(b_sol) = mtm.qr().solve(&mt_dv) {
            for (j, &(p, q)) in terms.iter().enumerate() {
                sip.b[p][q] = b_sol[j];
            }
        }

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

        if let Some(b_sol) = mtm.qr().solve(&mt_dv) {
            for (j, &(p, q)) in terms.iter().enumerate() {
                sip.b[p][q] = b_sol[j];
            }
        }

        sip
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
    }
}
