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
}
