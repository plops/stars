use crate::star_finder::DetectedStar;
use kiddo::KdTree;
use serde::{Deserialize, Serialize};

pub mod catalog;
pub use catalog::{load_catalog, CatalogStar};

pub mod sip;
pub use sip::SipDistortion;

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
    #[serde(default)]
    pub dx_pixels: f64,
    #[serde(default)]
    pub dy_pixels: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstrometricSolution {
    pub center_ra_deg: f64,
    pub center_dec_deg: f64,
    pub estimated_alt_deg: f64,
    pub focal_length_est_mm: f64,
    pub fov_deg: f64,
    pub solved: bool,
    pub matches: Vec<StarMatch>,
    pub rmse_pixels: f64,
    #[serde(default)]
    pub sip_distortion: Option<SipDistortion>,
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
    load_catalog()
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

    let denom = lat_rad.cos() * alt_rad.cos();
    let (cos_az, sin_az) = if denom.abs() < 1e-10 {
        // At zenith or poles, azimuth is undefined; default to 0
        (1.0, 0.0)
    } else {
        let ca = (dec_rad.sin() - lat_rad.sin() * sin_alt) / denom;
        let sa = -dec_rad.cos() * ha_rad.sin() / alt_rad.cos();
        (ca, sa)
    };

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
    altaz_to_pixel_with_refraction(
        alt_deg,
        az_deg,
        center_alt,
        center_az,
        focal_len_35mm,
        width,
        height,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn altaz_to_pixel_with_refraction(
    alt_deg: f64,
    az_deg: f64,
    center_alt: f64,
    center_az: f64,
    focal_len_35mm: f64,
    width: u32,
    height: u32,
    enable_refraction: bool,
) -> Option<(f64, f64)> {
    let effective_alt = if enable_refraction {
        alt_deg + crate::aberration::atmospheric_refraction_correction(alt_deg)
    } else {
        alt_deg
    };

    let d_az_rad = (az_deg - center_az).to_radians();
    let alt_rad = effective_alt.to_radians();
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
pub fn estimate_center_altitude(
    detected_stars: &[DetectedStar],
    catalog: &[CatalogStar],
    lat_deg: f64,
    lst: f64,
    center_az: f64,
    focal_len_35mm: f64,
    width: u32,
    height: u32,
) -> f64 {
    if detected_stars.is_empty() {
        return 45.0;
    }

    let mut tree_2d: KdTree<f64, 2> = KdTree::new();
    for (i, star) in detected_stars.iter().enumerate() {
        tree_2d.add(&[star.x, star.y], i as u64);
    }

    let count_matches_at_alt = |test_alt: f64| -> usize {
        let mut count = 0;
        let mut matched_ids = std::collections::HashSet::new();
        for cat in catalog {
            let (alt, az) = radec_to_altaz(cat.ra_deg, cat.dec_deg, lat_deg, lst);
            if let Some((px, py)) =
                altaz_to_pixel(alt, az, test_alt, center_az, focal_len_35mm, width, height)
            {
                let nearest = tree_2d.nearest_one::<kiddo::SquaredEuclidean>(&[px, py]);
                if nearest.distance.sqrt() < 25.0 {
                    let star_idx = nearest.item as usize;
                    if matched_ids.insert(star_idx) {
                        count += 1;
                    }
                }
            }
        }
        count
    };

    let coarse_alts = [20.0, 35.0, 45.0, 55.0, 70.0, 85.0];
    let mut best_alt = 45.0;
    let mut max_matches = 0;

    for &alt in &coarse_alts {
        let matches = count_matches_at_alt(alt);
        if matches > max_matches {
            max_matches = matches;
            best_alt = alt;
        }
    }

    let refine_offsets = [-5.0, -2.5, 2.5, 5.0];
    let mut refined_best_alt = best_alt;
    for &offset in &refine_offsets {
        let candidate_alt = (best_alt + offset).clamp(5.0, 89.0);
        let matches = count_matches_at_alt(candidate_alt);
        if matches > max_matches {
            max_matches = matches;
            refined_best_alt = candidate_alt;
        }
    }

    refined_best_alt
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

    let center_alt = estimate_center_altitude(
        detected_stars,
        &catalog,
        lat_deg,
        lst,
        heading_deg,
        focal_len_35mm,
        width,
        height,
    );
    let center_az = heading_deg;

    // 1. Build 2D KdTree for spatial pixel nearest neighbor matching
    let mut tree_2d: KdTree<f64, 2> = KdTree::new();
    for (i, star) in detected_stars.iter().enumerate() {
        tree_2d.add(&[star.x, star.y], i as u64);
    }

    // Pass 1: Coarse match with proper motion propagation and 25px radius limit
    let mut initial_matches = Vec::new();
    let mut matched_detected_ids = std::collections::HashSet::new();

    for cat in &catalog {
        let (ra_epoch, dec_epoch) = cat.position_at_epoch(timestamp_utc);
        let (alt, az) = radec_to_altaz(ra_epoch, dec_epoch, lat_deg, lst);
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

                if dist < 25.0 {
                    let star_idx = nearest.item as usize;
                    if !matched_detected_ids.contains(&star_idx) {
                        matched_detected_ids.insert(star_idx);
                        let det = &detected_stars[star_idx];

                        let dx = det.x - proj_x;
                        let dy = det.y - proj_y;
                        initial_matches.push(StarMatch {
                            star_id: det.id,
                            pixel_x: det.x,
                            pixel_y: det.y,
                            catalog_name: cat.name.clone(),
                            catalog_ra: ra_epoch,
                            catalog_dec: dec_epoch,
                            catalog_vmag: cat.vmag,
                            residual_pixels: dist,
                            dx_pixels: dx,
                            dy_pixels: dy,
                        });
                    }
                }
            }
        }
    }

    // Pass 2: Outlier rejection via 3-Sigma clipping
    let matches = if initial_matches.len() >= 4 {
        let mean_res: f64 = initial_matches
            .iter()
            .map(|m| m.residual_pixels)
            .sum::<f64>()
            / initial_matches.len() as f64;
        let variance: f64 = initial_matches
            .iter()
            .map(|m| (m.residual_pixels - mean_res).powi(2))
            .sum::<f64>()
            / initial_matches.len() as f64;
        let std_dev = variance.sqrt();

        let threshold = (mean_res + 2.5 * std_dev).min(18.0);
        let pruned: Vec<StarMatch> = initial_matches
            .into_iter()
            .filter(|m| m.residual_pixels <= threshold)
            .collect();
        if pruned.is_empty() {
            Vec::new()
        } else {
            pruned
        }
    } else {
        initial_matches
    };

    let sq_err_sum: f64 = matches.iter().map(|m| m.residual_pixels.powi(2)).sum();

    // 2. Perform Geometric 4D Quad Hashing Verification
    let mut quad_matches = 0;
    if detected_stars.len() >= 4 {
        let mut quad_tree_4d: KdTree<f64, 4> = KdTree::new();
        let mut catalog_quads = Vec::new();

        // Project top catalog stars to pixel space to construct catalog 4D quads
        // Sort by visual magnitude (brightest first) to select the most reliable stars
        let mut projected_cat: Vec<(f64, f64, f64)> = catalog
            .iter()
            .filter_map(|cat| {
                let (ra_epoch, dec_epoch) = cat.position_at_epoch(timestamp_utc);
                let (alt, az) = radec_to_altaz(ra_epoch, dec_epoch, lat_deg, lst);
                altaz_to_pixel(
                    alt,
                    az,
                    center_alt,
                    center_az,
                    focal_len_35mm,
                    width,
                    height,
                )
                .map(|(px, py)| (px, py, cat.vmag))
            })
            .collect();
        projected_cat.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        if projected_cat.len() >= 4 {
            let n_top = projected_cat.len().min(12);
            for i in 0..n_top {
                for j in (i + 1)..n_top {
                    for k in (j + 1)..n_top {
                        for l in (k + 1)..n_top {
                            if let Some(qh) = QuadHash::compute(
                                (projected_cat[i].0, projected_cat[i].1),
                                (projected_cat[j].0, projected_cat[j].1),
                                (projected_cat[k].0, projected_cat[k].1),
                                (projected_cat[l].0, projected_cat[l].1),
                            ) {
                                catalog_quads.push(qh);
                                quad_tree_4d.add(&qh.to_array(), catalog_quads.len() as u64 - 1);
                            }
                        }
                    }
                }
            }

            // Extract quads from top detected stars
            let mut sorted_detected: Vec<&crate::star_finder::DetectedStar> =
                detected_stars.iter().collect();
            sorted_detected.sort_by_key(|s| std::cmp::Reverse(s.peak_brightness));
            let top_detected: Vec<(f64, f64)> = sorted_detected
                .iter()
                .take(12)
                .map(|s| (s.x, s.y))
                .collect();

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
    let is_solved = (matches.len() >= 4 && rmse < 15.0)
        || (matches.len() >= 3 && quad_matches >= 1 && rmse < 15.0);

    let sip_distortion = if matches.len() >= 4 {
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let mut pairs = Vec::new();
        for m in &matches {
            if let Some(cat) = catalog.iter().find(|c| c.name == m.catalog_name) {
                let (ra_epoch, dec_epoch) = cat.position_at_epoch(timestamp_utc);
                let (alt, az) = radec_to_altaz(ra_epoch, dec_epoch, lat_deg, lst);
                if let Some((px_cat, py_cat)) = altaz_to_pixel(
                    alt,
                    az,
                    center_alt,
                    center_az,
                    focal_len_35mm,
                    width,
                    height,
                ) {
                    let u_cat = px_cat - cx;
                    let v_cat = py_cat - cy;
                    let u_det = m.pixel_x - cx;
                    let v_det = m.pixel_y - cy;
                    pairs.push(((u_cat, v_cat), (u_det, v_det)));
                }
            }
        }
        if pairs.len() >= 4 {
            Some(SipDistortion::fit_from_point_pairs(&pairs, 3))
        } else {
            None
        }
    } else {
        None
    };

    AstrometricSolution {
        center_ra_deg: if !matches.is_empty() {
            // Vector averaging for RA to handle 0°/360° wrap-around correctly
            let (sin_sum, cos_sum): (f64, f64) = matches
                .iter()
                .map(|m| m.catalog_ra.to_radians())
                .fold((0.0, 0.0), |(s, c), ra| (s + ra.sin(), c + ra.cos()));
            sin_sum.atan2(cos_sum).to_degrees().rem_euclid(360.0)
        } else {
            (lst - heading_deg + 360.0) % 360.0
        },
        center_dec_deg: if !matches.is_empty() {
            // Compute weighted mean declination from matched catalog stars
            let dec_sum: f64 = matches.iter().map(|m| m.catalog_dec).sum();
            dec_sum / matches.len() as f64
        } else {
            lat_deg // Fallback: assume looking at meridian → Dec ≈ latitude
        },
        estimated_alt_deg: center_alt,
        focal_length_est_mm: focal_len_35mm,
        fov_deg,
        solved: is_solved,
        matches,
        rmse_pixels: rmse,
        sip_distortion,
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

    #[test]
    fn test_altitude_refinement() {
        let catalog = get_bright_star_catalog();
        let lat_deg = 48.137;
        let lon_deg = 11.575;
        let heading_deg = 180.0;
        let timestamp_utc = 1700000000;
        let focal_len_35mm = 26.0;
        let width = 1000;
        let height = 1000;
        let target_alt = 70.0;

        let jd = julian_date(timestamp_utc);
        let gmst = greenwich_mean_sidereal_time(jd);
        let lst = local_sidereal_time(gmst, lon_deg);

        let mut detected_stars = Vec::new();
        let mut star_id = 0;
        for cat in &catalog {
            let (alt, az) = radec_to_altaz(cat.ra_deg, cat.dec_deg, lat_deg, lst);
            if let Some((px, py)) = altaz_to_pixel(
                alt,
                az,
                target_alt,
                heading_deg,
                focal_len_35mm,
                width,
                height,
            ) {
                detected_stars.push(DetectedStar {
                    id: star_id,
                    x: px,
                    y: py,
                    intensity: 1000.0,
                    peak_brightness: 255,
                    snr: 20.0,
                    fwhm: 3.0,
                    elongation: 1.0,
                });
                star_id += 1;
            }
        }

        let solution = solve_plate(
            &detected_stars,
            lat_deg,
            lon_deg,
            heading_deg,
            timestamp_utc,
            focal_len_35mm,
            width,
            height,
        );

        assert!(
            solution.solved,
            "Plate solve should succeed for synthetic image at alt=70°"
        );
        assert!(
            (solution.estimated_alt_deg - target_alt).abs() <= 5.0,
            "Estimated altitude {} should be close to target altitude {}",
            solution.estimated_alt_deg,
            target_alt
        );
    }
}
