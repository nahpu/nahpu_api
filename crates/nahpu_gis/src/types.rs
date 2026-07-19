use serde::{Deserialize, Serialize};

use crate::GisError;

/// Portable point fields used by NAHPU GIS import and export operations.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoordinateData {
    /// User-facing coordinate identifier.
    pub name_id: String,
    /// Optional descriptive notes.
    pub notes: Option<String>,
    /// WGS84 longitude in decimal degrees.
    pub decimal_longitude: Option<f64>,
    /// WGS84 latitude in decimal degrees.
    pub decimal_latitude: Option<f64>,
    /// Optional elevation in meters.
    pub elevation_in_meter: Option<f64>,
}

impl CoordinateData {
    pub(crate) fn validate_for_export(&self, index: usize) -> Result<(), GisError> {
        let label = if self.name_id.trim().is_empty() {
            format!("record {}", index + 1)
        } else {
            format!("record {} ('{}')", index + 1, self.name_id)
        };
        let longitude = self
            .decimal_longitude
            .ok_or_else(|| GisError::InvalidCoordinate(format!("{label} is missing longitude")))?;
        let latitude = self
            .decimal_latitude
            .ok_or_else(|| GisError::InvalidCoordinate(format!("{label} is missing latitude")))?;
        if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
            return Err(GisError::InvalidCoordinate(format!(
                "{label} longitude must be finite and between -180 and 180"
            )));
        }
        if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
            return Err(GisError::InvalidCoordinate(format!(
                "{label} latitude must be finite and between -90 and 90"
            )));
        }
        if self
            .elevation_in_meter
            .is_some_and(|value| !value.is_finite())
        {
            return Err(GisError::InvalidCoordinate(format!(
                "{label} elevation must be finite"
            )));
        }
        Ok(())
    }
}

/// WGS84 bounds ordered by named edges rather than positional values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeographicBounds {
    /// Western longitude.
    pub west: f64,
    /// Southern latitude.
    pub south: f64,
    /// Eastern longitude.
    pub east: f64,
    /// Northern latitude.
    pub north: f64,
}
