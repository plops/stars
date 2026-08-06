use anyhow::Result;
use chrono::{NaiveDateTime, TimeZone, Utc};
use exif::{In, Reader, Tag, Value};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Seek};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExifMetadata {
    pub datetime_original: Option<String>,
    pub timestamp_utc: Option<i64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub heading_deg: Option<f64>,
    pub focal_length_mm: Option<f64>,
    pub focal_length_in_35mm: Option<f64>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens_model: Option<String>,
    pub orientation: Option<u32>,
    pub iso: Option<u32>,
    pub exposure_time: Option<f64>,
    pub f_number: Option<f64>,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
}

impl ExifMetadata {
    pub fn dummy_iphone_metadata() -> Self {
        Self {
            datetime_original: Some("2026:08:05 22:30:00".to_string()),
            timestamp_utc: Some(1785969000), // 2026-08-05T22:30:00Z
            latitude: Some(48.137154),       // Munich, Germany
            longitude: Some(11.576124),
            altitude: Some(520.0),
            heading_deg: Some(180.0), // Facing South
            focal_length_mm: Some(5.7),
            focal_length_in_35mm: Some(26.0), // Standard iPhone camera ~26mm equiv
            make: Some("Apple".to_string()),
            model: Some("iPhone 15 Pro".to_string()),
            lens_model: Some("iPhone 15 Pro back camera 6.86mm f/1.78".to_string()),
            orientation: Some(1),
            iso: Some(1600),
            exposure_time: Some(1.0 / 3.0),
            f_number: Some(1.78),
            image_width: Some(4032),
            image_height: Some(3024),
        }
    }
}

pub fn parse_exif_bytes(bytes: &[u8]) -> Result<ExifMetadata> {
    let mut cursor = Cursor::new(bytes);
    parse_exif_from_reader(&mut cursor)
}

pub fn parse_exif_from_reader<R: std::io::BufRead + Seek>(reader: &mut R) -> Result<ExifMetadata> {
    let exif_reader = Reader::new();
    let exif = match exif_reader.read_from_container(reader) {
        Ok(e) => e,
        Err(_) => return Ok(ExifMetadata::default()),
    };

    let mut meta = ExifMetadata::default();

    // Camera & Lens Info
    if let Some(field) = exif.get_field(Tag::Make, In::PRIMARY) {
        meta.make = Some(
            field
                .display_value()
                .to_string()
                .trim_matches('"')
                .to_string(),
        );
    }
    if let Some(field) = exif.get_field(Tag::Model, In::PRIMARY) {
        meta.model = Some(
            field
                .display_value()
                .to_string()
                .trim_matches('"')
                .to_string(),
        );
    }
    if let Some(field) = exif.get_field(Tag::LensModel, In::PRIMARY) {
        meta.lens_model = Some(
            field
                .display_value()
                .to_string()
                .trim_matches('"')
                .to_string(),
        );
    }
    if let Some(field) = exif.get_field(Tag::Orientation, In::PRIMARY) {
        if let Value::Short(ref s) = field.value {
            if !s.is_empty() {
                meta.orientation = Some(s[0] as u32);
            }
        }
    }

    // Focal Length
    if let Some(field) = exif.get_field(Tag::FocalLength, In::PRIMARY) {
        if let Value::Rational(ref r) = field.value {
            if !r.is_empty() && r[0].denom != 0 {
                meta.focal_length_mm = Some(r[0].num as f64 / r[0].denom as f64);
            }
        }
    }
    if let Some(field) = exif.get_field(Tag::FocalLengthIn35mmFilm, In::PRIMARY) {
        if let Value::Short(ref s) = field.value {
            if !s.is_empty() {
                meta.focal_length_in_35mm = Some(s[0] as f64);
            }
        }
    }

    // ISO & Exposure
    if let Some(field) = exif.get_field(Tag::PhotographicSensitivity, In::PRIMARY) {
        if let Value::Short(ref s) = field.value {
            if !s.is_empty() {
                meta.iso = Some(s[0] as u32);
            }
        }
    }
    if let Some(field) = exif.get_field(Tag::ExposureTime, In::PRIMARY) {
        if let Value::Rational(ref r) = field.value {
            if !r.is_empty() && r[0].denom != 0 {
                meta.exposure_time = Some(r[0].num as f64 / r[0].denom as f64);
            }
        }
    }
    if let Some(field) = exif.get_field(Tag::FNumber, In::PRIMARY) {
        if let Value::Rational(ref r) = field.value {
            if !r.is_empty() && r[0].denom != 0 {
                meta.f_number = Some(r[0].num as f64 / r[0].denom as f64);
            }
        }
    }

    // DateTime
    if let Some(field) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
        let dt_str = field.display_value().to_string();
        meta.datetime_original = Some(dt_str.clone());
        // Try standard EXIF format (colons) first, then fallback to hyphens
        let dt_trimmed = dt_str.trim_matches('"');
        if let Ok(ndt) = NaiveDateTime::parse_from_str(dt_trimmed, "%Y:%m:%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(dt_trimmed, "%Y-%m-%d %H:%M:%S"))
        {
            meta.timestamp_utc = Some(Utc.from_utc_datetime(&ndt).timestamp());
        }
    }

    // GPS Latitude & Longitude
    let lat_ref = exif
        .get_field(Tag::GPSLatitudeRef, In::PRIMARY)
        .map(|f| f.display_value().to_string());
    let lat_field = exif.get_field(Tag::GPSLatitude, In::PRIMARY);
    if let (Some(lat_field), Some(lat_ref)) = (lat_field, lat_ref) {
        if let Value::Rational(ref r) = lat_field.value {
            if r.len() >= 3 && r[0].denom != 0 && r[1].denom != 0 && r[2].denom != 0 {
                let deg = r[0].num as f64 / r[0].denom as f64;
                let min = r[1].num as f64 / r[1].denom as f64;
                let sec = r[2].num as f64 / r[2].denom as f64;
                let mut lat = deg + (min / 60.0) + (sec / 3600.0);
                if lat_ref.contains('S') {
                    lat = -lat;
                }
                meta.latitude = Some(lat);
            }
        }
    }

    let lon_ref = exif
        .get_field(Tag::GPSLongitudeRef, In::PRIMARY)
        .map(|f| f.display_value().to_string());
    let lon_field = exif.get_field(Tag::GPSLongitude, In::PRIMARY);
    if let (Some(lon_field), Some(lon_ref)) = (lon_field, lon_ref) {
        if let Value::Rational(ref r) = lon_field.value {
            if r.len() >= 3 && r[0].denom != 0 && r[1].denom != 0 && r[2].denom != 0 {
                let deg = r[0].num as f64 / r[0].denom as f64;
                let min = r[1].num as f64 / r[1].denom as f64;
                let sec = r[2].num as f64 / r[2].denom as f64;
                let mut lon = deg + (min / 60.0) + (sec / 3600.0);
                if lon_ref.contains('W') {
                    lon = -lon;
                }
                meta.longitude = Some(lon);
            }
        }
    }

    // GPS Altitude
    if let Some(field) = exif.get_field(Tag::GPSAltitude, In::PRIMARY) {
        if let Value::Rational(ref r) = field.value {
            if !r.is_empty() && r[0].denom != 0 {
                meta.altitude = Some(r[0].num as f64 / r[0].denom as f64);
            }
        }
    }

    // GPS Image Direction (Heading)
    if let Some(field) = exif.get_field(Tag::GPSImgDirection, In::PRIMARY) {
        if let Value::Rational(ref r) = field.value {
            if !r.is_empty() && r[0].denom != 0 {
                meta.heading_deg = Some(r[0].num as f64 / r[0].denom as f64);
            }
        }
    }

    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_iphone_metadata() {
        let meta = ExifMetadata::dummy_iphone_metadata();
        assert_eq!(meta.make.as_deref(), Some("Apple"));
        assert_eq!(meta.model.as_deref(), Some("iPhone 15 Pro"));
        assert_eq!(meta.latitude, Some(48.137154));
        assert_eq!(meta.longitude, Some(11.576124));
        assert_eq!(meta.heading_deg, Some(180.0));
    }

    #[test]
    fn test_parse_empty_bytes() {
        let meta = parse_exif_bytes(&[]).unwrap();
        assert_eq!(meta, ExifMetadata::default());
    }
}
