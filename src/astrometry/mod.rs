use crate::star_finder::DetectedStar;
use kiddo::KdTree;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogStar {
    pub name: String,
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub vmag: f64,
    pub spectral: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarMatch {
    pub star_id: usize,
    pub pixel_x: f64,
    pub pixel_y: f64,
    pub catalog_name: String,
    pub catalog_ra: f64,
    pub catalog_dec: f64,
    pub catalog_vmag: f64,
    pub residual_pixels: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstrometricSolution {
    pub center_ra_deg: f64,
    pub center_dec_deg: f64,
    pub focal_length_est_mm: f64,
    pub fov_deg: f64,
    pub solved: bool,
    pub matches: Vec<StarMatch>,
    pub rmse_pixels: f64,
}

/// 4D Geometric Quad Hash for Lost-in-Space Plate Solving
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadHash {
    pub u1: f64,
    pub v1: f64,
    pub u2: f64,
    pub v2: f64,
}

impl QuadHash {
    /// Compute scale and rotation invariant 4D hash from 4 2D point positions
    pub fn compute(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), p4: (f64, f64)) -> Option<Self> {
        let pts = [p1, p2, p3, p4];
        let mut max_dist_sq = 0.0;
        let mut idx_a = 0;
        let mut idx_b = 1;

        // Find the pair with maximum distance as the basis
        for i in 0..4 {
            for j in (i + 1)..4 {
                let dx = pts[j].0 - pts[i].0;
                let dy = pts[j].1 - pts[i].1;
                let d2 = dx * dx + dy * dy;
                if d2 > max_dist_sq {
                    max_dist_sq = d2;
                    idx_a = i;
                    idx_b = j;
                }
            }
        }

        if max_dist_sq < 1e-6 {
            return None;
        }

        let pa = pts[idx_a];
        let pb = pts[idx_b];
        let dx_ab = pb.0 - pa.0;
        let dy_ab = pb.1 - pa.1;

        let rem_indices: Vec<usize> = (0..4).filter(|&k| k != idx_a && k != idx_b).collect();
        if rem_indices.len() != 2 {
            return None;
        }

        let transform = |p: (f64, f64)| -> (f64, f64) {
            let dx = p.0 - pa.0;
            let dy = p.1 - pa.1;
            let u = (dx * dx_ab + dy * dy_ab) / max_dist_sq;
            let v = (-dx * dy_ab + dy * dx_ab) / max_dist_sq;
            (u, v)
        };

        let mut c1 = transform(pts[rem_indices[0]]);
        let mut c2 = transform(pts[rem_indices[1]]);

        // Order residual points canonically so u1 <= u2
        if c1.0 > c2.0 {
            std::mem::swap(&mut c1, &mut c2);
        }

        Some(QuadHash {
            u1: c1.0,
            v1: c1.1,
            u2: c2.0,
            v2: c2.1,
        })
    }

    pub fn to_array(self) -> [f64; 4] {
        [self.u1, self.v1, self.u2, self.v2]
    }
}

pub fn get_bright_star_catalog() -> Vec<CatalogStar> {
    vec![
        CatalogStar {
            name: "Sirius (α CMa)".into(),
            ra_deg: 101.287,
            dec_deg: -16.716,
            vmag: -1.46,
            spectral: "A1V".into(),
        },
        CatalogStar {
            name: "Canopus (α Car)".into(),
            ra_deg: 95.988,
            dec_deg: -52.696,
            vmag: -0.74,
            spectral: "F0II".into(),
        },
        CatalogStar {
            name: "Arcturus (α Boo)".into(),
            ra_deg: 213.915,
            dec_deg: 19.182,
            vmag: -0.05,
            spectral: "K1.5III".into(),
        },
        CatalogStar {
            name: "Rigel (β Ori)".into(),
            ra_deg: 78.634,
            dec_deg: -8.202,
            vmag: 0.13,
            spectral: "B8Ia".into(),
        },
        CatalogStar {
            name: "Betelgeuse (α Ori)".into(),
            ra_deg: 88.793,
            dec_deg: 7.407,
            vmag: 0.50,
            spectral: "M1Ia".into(),
        },
        CatalogStar {
            name: "Capella (α Aur)".into(),
            ra_deg: 79.172,
            dec_deg: 45.998,
            vmag: 0.08,
            spectral: "G3III".into(),
        },
        CatalogStar {
            name: "Vega (α Lyr)".into(),
            ra_deg: 279.234,
            dec_deg: 38.784,
            vmag: 0.03,
            spectral: "A0V".into(),
        },
        CatalogStar {
            name: "Procyon (α CMi)".into(),
            ra_deg: 114.825,
            dec_deg: 5.225,
            vmag: 0.37,
            spectral: "F5IV-V".into(),
        },
        CatalogStar {
            name: "Altair (α Aql)".into(),
            ra_deg: 297.696,
            dec_deg: 8.868,
            vmag: 0.77,
            spectral: "A7V".into(),
        },
        CatalogStar {
            name: "Aldebaran (α Tau)".into(),
            ra_deg: 68.980,
            dec_deg: 16.509,
            vmag: 0.85,
            spectral: "K5III".into(),
        },
        CatalogStar {
            name: "Spica (α Vir)".into(),
            ra_deg: 201.298,
            dec_deg: -11.161,
            vmag: 0.98,
            spectral: "B1III-IV".into(),
        },
        CatalogStar {
            name: "Antares (α Sco)".into(),
            ra_deg: 247.352,
            dec_deg: -26.432,
            vmag: 1.06,
            spectral: "M1.5Iab".into(),
        },
        CatalogStar {
            name: "Fomalhaut (α PsA)".into(),
            ra_deg: 344.413,
            dec_deg: -29.622,
            vmag: 1.17,
            spectral: "A3V".into(),
        },
        CatalogStar {
            name: "Deneb (α Cyg)".into(),
            ra_deg: 310.358,
            dec_deg: 45.280,
            vmag: 1.25,
            spectral: "A2Ia".into(),
        },
        CatalogStar {
            name: "Regulus (α Leo)".into(),
            ra_deg: 152.093,
            dec_deg: 11.967,
            vmag: 1.36,
            spectral: "B8IVn".into(),
        },
        CatalogStar {
            name: "Castor (α Gem)".into(),
            ra_deg: 113.650,
            dec_deg: 31.888,
            vmag: 1.58,
            spectral: "A1V".into(),
        },
        CatalogStar {
            name: "Pollux (β Gem)".into(),
            ra_deg: 116.329,
            dec_deg: 28.026,
            vmag: 1.14,
            spectral: "K0III".into(),
        },
        CatalogStar {
            name: "Polaris (α UMi)".into(),
            ra_deg: 37.954,
            dec_deg: 89.264,
            vmag: 1.98,
            spectral: "F7Ib".into(),
        },
        CatalogStar {
            name: "Alnitak (ζ Ori)".into(),
            ra_deg: 85.190,
            dec_deg: -1.943,
            vmag: 1.77,
            spectral: "O9.5Ib".into(),
        },
        CatalogStar {
            name: "Alnilam (ε Ori)".into(),
            ra_deg: 84.053,
            dec_deg: -1.202,
            vmag: 1.69,
            spectral: "B0Ia".into(),
        },
        CatalogStar {
            name: "Mintaka (δ Ori)".into(),
            ra_deg: 83.002,
            dec_deg: -0.299,
            vmag: 2.23,
            spectral: "O9.5II".into(),
        },
        CatalogStar {
            name: "Bellatrix (γ Ori)".into(),
            ra_deg: 81.283,
            dec_deg: 6.349,
            vmag: 1.64,
            spectral: "B2III".into(),
        },
        CatalogStar {
            name: "Saiph (κ Ori)".into(),
            ra_deg: 86.939,
            dec_deg: -9.669,
            vmag: 2.07,
            spectral: "B0.5Ia".into(),
        },
        CatalogStar {
            name: "Dubhe (α UMa)".into(),
            ra_deg: 165.932,
            dec_deg: 61.751,
            vmag: 1.79,
            spectral: "K0III".into(),
        },
        CatalogStar {
            name: "Merak (β UMa)".into(),
            ra_deg: 165.460,
            dec_deg: 56.382,
            vmag: 2.37,
            spectral: "A1V".into(),
        },
        CatalogStar {
            name: "Phecda (γ UMa)".into(),
            ra_deg: 178.458,
            dec_deg: 53.695,
            vmag: 2.44,
            spectral: "A0Ve".into(),
        },
        CatalogStar {
            name: "Megrez (δ UMa)".into(),
            ra_deg: 183.857,
            dec_deg: 57.032,
            vmag: 3.31,
            spectral: "A3V".into(),
        },
        CatalogStar {
            name: "Alioth (ε UMa)".into(),
            ra_deg: 193.507,
            dec_deg: 55.959,
            vmag: 1.77,
            spectral: "A1p".into(),
        },
        CatalogStar {
            name: "Mizar (ζ UMa)".into(),
            ra_deg: 200.981,
            dec_deg: 54.925,
            vmag: 2.23,
            spectral: "A2V".into(),
        },
        CatalogStar {
            name: "Alkaid (η UMa)".into(),
            ra_deg: 206.885,
            dec_deg: 49.313,
            vmag: 1.86,
            spectral: "B3V".into(),
        },
        CatalogStar {
            name: "Algol (β Per)".into(),
            ra_deg: 47.042,
            dec_deg: 40.956,
            vmag: 2.12,
            spectral: "B8V".into(),
        },
        CatalogStar {
            name: "Mirfak (α Per)".into(),
            ra_deg: 51.081,
            dec_deg: 49.861,
            vmag: 1.79,
            spectral: "F5Ib".into(),
        },
        CatalogStar {
            name: "Hamal (α Ari)".into(),
            ra_deg: 31.793,
            dec_deg: 23.462,
            vmag: 2.01,
            spectral: "K2III".into(),
        },
        CatalogStar {
            name: "Denebola (β Leo)".into(),
            ra_deg: 177.265,
            dec_deg: 14.572,
            vmag: 2.14,
            spectral: "A3V".into(),
        },
        CatalogStar {
            name: "Alphard (α Hya)".into(),
            ra_deg: 141.897,
            dec_deg: -8.661,
            vmag: 1.98,
            spectral: "K3II-III".into(),
        },
        CatalogStar {
            name: "Rasalhague (α Oph)".into(),
            ra_deg: 263.738,
            dec_deg: 12.560,
            vmag: 2.08,
            spectral: "A5III".into(),
        },
        CatalogStar {
            name: "Tarazed (γ Aql)".into(),
            ra_deg: 298.058,
            dec_deg: 10.613,
            vmag: 2.72,
            spectral: "K3II".into(),
        },
        CatalogStar {
            name: "Alshain (β Aql)".into(),
            ra_deg: 298.828,
            dec_deg: 6.405,
            vmag: 3.71,
            spectral: "G8IV".into(),
        },
        CatalogStar {
            name: "Rotanev (β Del)".into(),
            ra_deg: 309.047,
            dec_deg: 14.595,
            vmag: 3.63,
            spectral: "F5IV".into(),
        },
        CatalogStar {
            name: "Sualocin (α Del)".into(),
            ra_deg: 309.964,
            dec_deg: 15.908,
            vmag: 3.77,
            spectral: "B9V".into(),
        },
        CatalogStar {
            name: "Sadalmelik (α Aqr)".into(),
            ra_deg: 331.044,
            dec_deg: -0.319,
            vmag: 2.95,
            spectral: "G2Ib".into(),
        },
        CatalogStar {
            name: "Sadalsuud (β Aqr)".into(),
            ra_deg: 323.914,
            dec_deg: -5.571,
            vmag: 2.87,
            spectral: "G0Ib".into(),
        },
        CatalogStar {
            name: "Scheat (β Peg)".into(),
            ra_deg: 345.944,
            dec_deg: 28.083,
            vmag: 2.42,
            spectral: "M2.5II-III".into(),
        },
        CatalogStar {
            name: "Markab (α Peg)".into(),
            ra_deg: 346.190,
            dec_deg: 15.205,
            vmag: 2.49,
            spectral: "B9V".into(),
        },
        CatalogStar {
            name: "Algenib (γ Peg)".into(),
            ra_deg: 3.309,
            dec_deg: 15.183,
            vmag: 2.84,
            spectral: "B2IV".into(),
        },
        CatalogStar {
            name: "Enif (ε Peg)".into(),
            ra_deg: 325.990,
            dec_deg: 9.875,
            vmag: 2.38,
            spectral: "K2Ib".into(),
        },
        CatalogStar {
            name: "Kochab (β UMi)".into(),
            ra_deg: 222.676,
            dec_deg: 74.156,
            vmag: 2.08,
            spectral: "K4III".into(),
        },
        CatalogStar {
            name: "Sadr (γ Cyg)".into(),
            ra_deg: 305.557,
            dec_deg: 40.257,
            vmag: 2.23,
            spectral: "F8Ib".into(),
        },
        CatalogStar {
            name: "Albireo (β1 Cyg)".into(),
            ra_deg: 292.680,
            dec_deg: 27.960,
            vmag: 3.05,
            spectral: "K3II".into(),
        },
        CatalogStar {
            name: "Navi (γ Cas)".into(),
            ra_deg: 14.177,
            dec_deg: 60.717,
            vmag: 2.15,
            spectral: "B0IVe".into(),
        },
    ]
}

// Convert UTC timestamp to Julian Date
pub fn julian_date(timestamp_utc: i64) -> f64 {
    2440587.5 + (timestamp_utc as f64 / 86400.0)
}

// Greenwich Mean Sidereal Time in degrees
pub fn greenwich_mean_sidereal_time(jd: f64) -> f64 {
    let d = jd - 2451545.0;
    let gmst = 280.46061837 + 360.98564736629 * d;
    (gmst % 360.0 + 360.0) % 360.0
}

// Local Sidereal Time in degrees
pub fn local_sidereal_time(gmst_deg: f64, lon_deg: f64) -> f64 {
    (gmst_deg + lon_deg % 360.0 + 360.0) % 360.0
}

// Equatorial (RA, Dec) to Horizontal (Alt, Az) transformation
pub fn radec_to_altaz(ra_deg: f64, dec_deg: f64, lat_deg: f64, lst_deg: f64) -> (f64, f64) {
    let ha_rad = (lst_deg - ra_deg).to_radians();
    let dec_rad = dec_deg.to_radians();
    let lat_rad = lat_deg.to_radians();

    let sin_alt = lat_rad.sin() * dec_rad.sin() + lat_rad.cos() * dec_rad.cos() * ha_rad.cos();
    let alt_rad = sin_alt.asin();

    let cos_az = (dec_rad.sin() - lat_rad.sin() * sin_alt) / (lat_rad.cos() * alt_rad.cos());
    let sin_az = -dec_rad.cos() * ha_rad.sin() / alt_rad.cos();

    let mut az_rad = cos_az.clamp(-1.0, 1.0).acos();
    if sin_az < 0.0 {
        az_rad = 2.0 * std::f64::consts::PI - az_rad;
    }

    (alt_rad.to_degrees(), az_rad.to_degrees())
}

// Project Horizontal (Alt, Az) into camera image pixel (X, Y)
pub fn altaz_to_pixel(
    alt_deg: f64,
    az_deg: f64,
    center_alt: f64,
    center_az: f64,
    focal_len_35mm: f64,
    width: u32,
    height: u32,
) -> Option<(f64, f64)> {
    let d_az_rad = (az_deg - center_az).to_radians();
    let alt_rad = alt_deg.to_radians();
    let c_alt_rad = center_alt.to_radians();

    let cos_c = c_alt_rad.sin() * alt_rad.sin() + c_alt_rad.cos() * alt_rad.cos() * d_az_rad.cos();
    if cos_c <= 0.0 {
        return None; // Star is behind camera field of view
    }

    let x_proj = alt_rad.cos() * d_az_rad.sin() / cos_c;
    let y_proj = (c_alt_rad.cos() * alt_rad.sin()
        - c_alt_rad.sin() * alt_rad.cos() * d_az_rad.cos())
        / cos_c;

    // Standard 35mm film frame width = 36mm
    let scale_factor = (width as f64) * (focal_len_35mm / 36.0);

    let px = (width as f64 / 2.0) + x_proj * scale_factor;
    let py = (height as f64 / 2.0) - y_proj * scale_factor;

    if px >= 0.0 && px < width as f64 && py >= 0.0 && py < height as f64 {
        Some((px, py))
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
pub fn solve_plate(
    detected_stars: &[DetectedStar],
    lat_deg: f64,
    lon_deg: f64,
    heading_deg: f64,
    timestamp_utc: i64,
    focal_len_35mm: f64,
    width: u32,
    height: u32,
) -> AstrometricSolution {
    let catalog = get_bright_star_catalog();
    let jd = julian_date(timestamp_utc);
    let gmst = greenwich_mean_sidereal_time(jd);
    let lst = local_sidereal_time(gmst, lon_deg);

    let center_alt = 45.0; // Altitude angle estimate facing sky
    let center_az = heading_deg;

    // 1. Build 2D KdTree for spatial pixel nearest neighbor matching
    let mut tree_2d: KdTree<f64, 2> = KdTree::new();
    for (i, star) in detected_stars.iter().enumerate() {
        tree_2d.add(&[star.x, star.y], i as u64);
    }

    let mut matches = Vec::new();
    let mut sq_err_sum = 0.0;

    for cat in &catalog {
        let (alt, az) = radec_to_altaz(cat.ra_deg, cat.dec_deg, lat_deg, lst);
        if let Some((proj_x, proj_y)) = altaz_to_pixel(
            alt,
            az,
            center_alt,
            center_az,
            focal_len_35mm,
            width,
            height,
        ) {
            if !detected_stars.is_empty() {
                let nearest = tree_2d.nearest_one::<kiddo::SquaredEuclidean>(&[proj_x, proj_y]);
                let dist = nearest.distance.sqrt();

                if dist < 120.0 {
                    let star_idx = nearest.item as usize;
                    let det = &detected_stars[star_idx];

                    sq_err_sum += dist * dist;
                    matches.push(StarMatch {
                        star_id: det.id,
                        pixel_x: det.x,
                        pixel_y: det.y,
                        catalog_name: cat.name.clone(),
                        catalog_ra: cat.ra_deg,
                        catalog_dec: cat.dec_deg,
                        catalog_vmag: cat.vmag,
                        residual_pixels: dist,
                    });
                }
            }
        }
    }

    // 2. Perform Geometric 4D Quad Hashing Verification
    let mut quad_matches = 0;
    if detected_stars.len() >= 4 {
        let mut quad_tree_4d: KdTree<f64, 4> = KdTree::new();
        let mut catalog_quads = Vec::new();

        // Project top catalog stars to pixel space to construct catalog 4D quads
        let projected_cat: Vec<(f64, f64)> = catalog
            .iter()
            .filter_map(|cat| {
                let (alt, az) = radec_to_altaz(cat.ra_deg, cat.dec_deg, lat_deg, lst);
                altaz_to_pixel(
                    alt,
                    az,
                    center_alt,
                    center_az,
                    focal_len_35mm,
                    width,
                    height,
                )
            })
            .collect();

        if projected_cat.len() >= 4 {
            for i in 0..projected_cat.len().min(12) {
                for j in (i + 1)..projected_cat.len().min(12) {
                    for k in (j + 1)..projected_cat.len().min(12) {
                        for l in (k + 1)..projected_cat.len().min(12) {
                            if let Some(qh) = QuadHash::compute(
                                projected_cat[i],
                                projected_cat[j],
                                projected_cat[k],
                                projected_cat[l],
                            ) {
                                catalog_quads.push(qh);
                                quad_tree_4d.add(&qh.to_array(), catalog_quads.len() as u64 - 1);
                            }
                        }
                    }
                }
            }

            // Extract quads from top detected stars
            let top_detected: Vec<(f64, f64)> =
                detected_stars.iter().take(12).map(|s| (s.x, s.y)).collect();

            for i in 0..top_detected.len() {
                for j in (i + 1)..top_detected.len() {
                    for k in (j + 1)..top_detected.len() {
                        for l in (k + 1)..top_detected.len() {
                            if let Some(img_qh) = QuadHash::compute(
                                top_detected[i],
                                top_detected[j],
                                top_detected[k],
                                top_detected[l],
                            ) {
                                if !catalog_quads.is_empty() {
                                    let near = quad_tree_4d
                                        .nearest_one::<kiddo::SquaredEuclidean>(&img_qh.to_array());
                                    if near.distance.sqrt() < 0.08 {
                                        quad_matches += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let rmse = if !matches.is_empty() {
        (sq_err_sum / matches.len() as f64).sqrt()
    } else {
        0.0
    };

    let fov_deg = 2.0 * ((18.0 / focal_len_35mm).atan()).to_degrees();
    let is_solved = matches.len() >= 3 || quad_matches >= 2;

    AstrometricSolution {
        center_ra_deg: (lst - heading_deg % 360.0 + 360.0) % 360.0,
        center_dec_deg: 10.0,
        focal_length_est_mm: focal_len_35mm,
        fov_deg,
        solved: is_solved,
        matches,
        rmse_pixels: rmse,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_julian_date() {
        let jd = julian_date(946728000);
        assert!((jd - 2451545.0).abs() < 0.001);
    }

    #[test]
    fn test_radec_to_altaz() {
        let (alt, az) = radec_to_altaz(88.793, 7.407, 48.137, 88.793);
        assert!(alt > 0.0, "Betelgeuse should be above horizon at LST=RA");
        assert!(
            (az - 180.0).abs() < 10.0,
            "At LST=RA, star should be near due South (az=180)"
        );
    }

    #[test]
    fn test_quad_hash_invariance() {
        let p1 = (100.0, 100.0);
        let p2 = (300.0, 150.0);
        let p3 = (150.0, 300.0);
        let p4 = (400.0, 400.0);

        let q1 = QuadHash::compute(p1, p2, p3, p4).expect("Quad hash computed");

        let rotate_scale =
            |(x, y): (f64, f64)| -> (f64, f64) { (y * 2.5 + 50.0, -x * 2.5 + 500.0) };

        let q2 = QuadHash::compute(
            rotate_scale(p1),
            rotate_scale(p2),
            rotate_scale(p3),
            rotate_scale(p4),
        )
        .expect("Rotated quad hash computed");

        let err = (q1.u1 - q2.u1).abs()
            + (q1.v1 - q2.v1).abs()
            + (q1.u2 - q2.u2).abs()
            + (q1.v2 - q2.v2).abs();
        assert!(
            err < 1e-4,
            "Quad hash must be scale and rotation invariant, error: {err}"
        );
    }
}
