use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogStar {
    pub name: String,
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub vmag: f64,
    pub spectral: String,
    #[serde(default)]
    pub parallax_mas: f64,
    #[serde(default)]
    pub pmra_mas: f64,
    #[serde(default)]
    pub pmdec_mas: f64,
}

impl CatalogStar {
    /// Propagate (RA, Dec) coordinates from Hipparcos epoch (J1991.25) to observation timestamp UTC
    pub fn position_at_epoch(&self, timestamp_utc: i64) -> (f64, f64) {
        let jd = 2440587.5 + (timestamp_utc as f64 / 86400.0);
        let dt_years = (jd - 2448349.0625) / 365.25;

        let cos_dec = self.dec_deg.to_radians().cos().max(1e-6);
        let dra_deg = (self.pmra_mas / (1000.0 * 3600.0 * cos_dec)) * dt_years;
        let ddec_deg = (self.pmdec_mas / (1000.0 * 3600.0)) * dt_years;

        let ra = (self.ra_deg + dra_deg).rem_euclid(360.0);
        let dec = (self.dec_deg + ddec_deg).clamp(-90.0, 90.0);

        (ra, dec)
    }
}

pub const EMBEDDED_CATALOG_CSV: &[u8] = include_bytes!("../../data/bright_stars.csv");

/// Load star catalog from `data/bright_stars.csv` or fallback to embedded CSV
pub fn load_catalog() -> Vec<CatalogStar> {
    if let Ok(content) = std::fs::read_to_string("data/bright_stars.csv") {
        if let Ok(stars) = parse_catalog_csv(content.as_bytes()) {
            if !stars.is_empty() {
                return stars;
            }
        }
    }

    parse_catalog_csv(EMBEDDED_CATALOG_CSV).unwrap_or_default()
}

pub fn parse_catalog_csv(csv_bytes: &[u8]) -> Result<Vec<CatalogStar>, csv::Error> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    let mut stars = Vec::new();
    for result in reader.deserialize() {
        let record: CatalogStar = result?;
        stars.push(record);
    }
    Ok(stars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_loading() {
        let catalog = load_catalog();
        assert!(
            catalog.len() >= 5000,
            "Expected >= 5000 stars, got {}",
            catalog.len()
        );

        let min_vmag = catalog.iter().map(|s| s.vmag).fold(f64::INFINITY, f64::min);
        let max_vmag = catalog
            .iter()
            .map(|s| s.vmag)
            .fold(f64::NEG_INFINITY, f64::max);

        assert!(
            min_vmag < 0.0,
            "Expected min magnitude < 0.0 (e.g. Sirius), got {min_vmag}"
        );
        assert!(
            max_vmag <= 6.5,
            "Expected max magnitude <= 6.5, got {max_vmag}"
        );
    }

    #[test]
    fn test_proper_motion_propagation() {
        let star = CatalogStar {
            name: "Barnard's Star Fake".to_string(),
            ra_deg: 100.0,
            dec_deg: 20.0,
            vmag: 5.0,
            spectral: "M5V".to_string(),
            parallax_mas: 500.0,
            pmra_mas: -800.0,   // -0.8 arcsec/yr
            pmdec_mas: 10000.0, // +10.0 arcsec/yr
        };

        // Timestamp for J2026.0 (~34.75 years after J1991.25)
        let ts_2026 = 1767225600; // 2026-01-01T00:00:00Z
        let (ra, dec) = star.position_at_epoch(ts_2026);

        // 34.75 yrs * 10 arcsec/yr = 347.5 arcsec = 0.096527 deg increase in Dec
        assert!((dec - 20.0965).abs() < 0.005);
        assert!(ra < 100.0); // negative pmra moves RA lower
    }
}
