//! A module for geographic coordinate conversions.
//!
//! Provides types and methods for converting between decimal degrees and DMS
//! (Degrees, Minutes, Seconds), DDM (Degrees, Decimal Minutes), and UTM
//! (Universal Transverse Mercator) coordinate systems.

use serde::{Deserialize, Serialize};

use crate::GisError;

/// Represents a geographic cardinal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardinalDirection {
    /// Northern hemisphere (positive latitude)
    North,
    /// Southern hemisphere (negative latitude)
    South,
    /// Eastern hemisphere (positive longitude)
    East,
    /// Western hemisphere (negative longitude)
    West,
}

impl CardinalDirection {
    /// Checks if this direction represents a negative sign (South or West).
    pub fn is_negative(&self) -> bool {
        matches!(self, Self::South | Self::West)
    }
}

/// Represents a coordinate in Degrees, Minutes, and Seconds (DMS) format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DmsCoordinate {
    /// Degree component of the coordinate.
    pub degrees: u32,
    /// Minute component of the coordinate.
    pub minutes: u32,
    /// Second component of the coordinate.
    pub seconds: f64,
    /// Cardinal direction of the coordinate.
    pub direction: CardinalDirection,
}

impl DmsCoordinate {
    /// Creates a new `DmsCoordinate` with the given components.
    pub fn new(degrees: u32, minutes: u32, seconds: f64, direction: CardinalDirection) -> Self {
        Self {
            degrees,
            minutes,
            seconds,
            direction,
        }
    }

    /// Converts the DMS coordinate to its decimal degrees representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use nahpu_gis::conversion::{DmsCoordinate, CardinalDirection};
    ///
    /// let coord = DmsCoordinate::new(41, 24, 12.2, CardinalDirection::North);
    /// assert!((coord.to_decimal() - 41.40338888888889).abs() < 1e-9);
    /// ```
    pub fn to_decimal(&self) -> f64 {
        let decimal = self.degrees as f64 + (self.minutes as f64 / 60.0) + (self.seconds / 3600.0);
        if self.direction.is_negative() {
            -decimal
        } else {
            decimal
        }
    }

    /// Validates component ranges for the coordinate direction.
    pub fn validate(&self) -> Result<(), GisError> {
        if self.minutes >= 60 {
            return Err(GisError::InvalidCoordinate(
                "DMS minutes must be less than 60".to_owned(),
            ));
        }
        if !self.seconds.is_finite() || !(0.0..60.0).contains(&self.seconds) {
            return Err(GisError::InvalidCoordinate(
                "DMS seconds must be finite and less than 60".to_owned(),
            ));
        }
        validate_degrees(
            self.degrees,
            self.minutes as f64,
            self.seconds,
            self.direction,
        )
    }

    /// Converts a decimal coordinate and direction into a `DmsCoordinate`.
    ///
    /// # Examples
    ///
    /// ```
    /// use nahpu_gis::conversion::{DmsCoordinate, CardinalDirection};
    ///
    /// let coord = DmsCoordinate::from_decimal(41.40338888888889, CardinalDirection::North);
    /// assert_eq!(coord.degrees, 41);
    /// assert_eq!(coord.minutes, 24);
    /// assert!((coord.seconds - 12.2).abs() < 1e-9);
    /// ```
    pub fn from_decimal(decimal: f64, direction: CardinalDirection) -> Self {
        let abs_val = decimal.abs();
        let degrees = abs_val.floor() as u32;
        let minutes_float = (abs_val - degrees as f64) * 60.0;
        let minutes = minutes_float.floor() as u32;
        let seconds = (minutes_float - minutes as f64) * 60.0;
        Self {
            degrees,
            minutes,
            seconds,
            direction,
        }
    }
}

impl std::str::FromStr for DmsCoordinate {
    type Err = GisError;

    /// Parses a DMS coordinate from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use nahpu_gis::conversion::{DmsCoordinate, CardinalDirection};
    ///
    /// let coord: DmsCoordinate = "41° 24' 12.2\" N".parse().unwrap();
    /// assert_eq!(coord.degrees, 41);
    /// assert_eq!(coord.minutes, 24);
    /// assert!((coord.seconds - 12.2).abs() < 1e-9);
    /// assert_eq!(coord.direction, CardinalDirection::North);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.trim();
        let clean_upper = clean.to_ascii_uppercase();

        let tokens = tokenize_positive_numbers(clean);
        if tokens.len() != 3 {
            return Err(GisError::Data(
                "DMS format requires exactly 3 numeric components".to_owned(),
            ));
        }

        let degrees = tokens[0] as u32;
        let minutes = tokens[1] as u32;
        let seconds = tokens[2];

        let direction = if clean_upper.contains('S') {
            CardinalDirection::South
        } else if clean_upper.contains('W') {
            CardinalDirection::West
        } else if clean_upper.contains('E') {
            CardinalDirection::East
        } else {
            CardinalDirection::North
        };

        let coordinate = Self {
            degrees,
            minutes,
            seconds,
            direction,
        };
        coordinate.validate()?;
        Ok(coordinate)
    }
}

/// Represents a coordinate in Degrees and Decimal Minutes (DDM) format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DdmCoordinate {
    /// Degree component of the coordinate.
    pub degrees: u32,
    /// Minute component of the coordinate.
    pub minutes: f64,
    /// Cardinal direction of the coordinate.
    pub direction: CardinalDirection,
}

impl DdmCoordinate {
    /// Creates a new `DdmCoordinate` with the given components.
    pub fn new(degrees: u32, minutes: f64, direction: CardinalDirection) -> Self {
        Self {
            degrees,
            minutes,
            direction,
        }
    }

    /// Converts the DDM coordinate to its decimal degrees representation.
    pub fn to_decimal(&self) -> f64 {
        let decimal = self.degrees as f64 + (self.minutes / 60.0);
        if self.direction.is_negative() {
            -decimal
        } else {
            decimal
        }
    }

    /// Validates component ranges for the coordinate direction.
    pub fn validate(&self) -> Result<(), GisError> {
        if !self.minutes.is_finite() || !(0.0..60.0).contains(&self.minutes) {
            return Err(GisError::InvalidCoordinate(
                "DDM minutes must be finite and less than 60".to_owned(),
            ));
        }
        validate_degrees(self.degrees, self.minutes, 0.0, self.direction)
    }

    /// Converts a decimal coordinate and direction into a `DdmCoordinate`.
    pub fn from_decimal(decimal: f64, direction: CardinalDirection) -> Self {
        let abs_val = decimal.abs();
        let degrees = abs_val.floor() as u32;
        let minutes = (abs_val - degrees as f64) * 60.0;
        Self {
            degrees,
            minutes,
            direction,
        }
    }
}

impl std::str::FromStr for DdmCoordinate {
    type Err = GisError;

    /// Parses a DDM coordinate from a string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.trim();
        let clean_upper = clean.to_ascii_uppercase();

        let tokens = tokenize_positive_numbers(clean);
        if tokens.len() != 2 {
            return Err(GisError::Data(
                "DDM format requires exactly 2 numeric components".to_owned(),
            ));
        }

        let degrees = tokens[0] as u32;
        let minutes = tokens[1];

        let direction = if clean_upper.contains('S') {
            CardinalDirection::South
        } else if clean_upper.contains('W') {
            CardinalDirection::West
        } else if clean_upper.contains('E') {
            CardinalDirection::East
        } else {
            CardinalDirection::North
        };

        let coordinate = Self {
            degrees,
            minutes,
            direction,
        };
        coordinate.validate()?;
        Ok(coordinate)
    }
}

/// Represents a coordinate in Universal Transverse Mercator (UTM) format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtmCoordinate {
    /// The UTM zone number (1-60).
    pub zone: u8,
    /// The UTM zone hemisphere (North or South).
    pub hemisphere: CardinalDirection,
    /// The Easting component in meters.
    pub easting: f64,
    /// The Northing component in meters.
    pub northing: f64,
}

impl UtmCoordinate {
    /// Creates a new `UtmCoordinate`.
    pub fn new(
        zone: u8,
        hemisphere: CardinalDirection,
        easting: f64,
        northing: f64,
    ) -> Result<Self, GisError> {
        if zone == 0 || zone > 60 {
            return Err(GisError::InvalidCoordinate(
                "UTM zone must be between 1 and 60".to_owned(),
            ));
        }
        if !matches!(
            hemisphere,
            CardinalDirection::North | CardinalDirection::South
        ) {
            return Err(GisError::InvalidCoordinate(
                "UTM hemisphere must be North or South".to_owned(),
            ));
        }
        if !easting.is_finite() || !northing.is_finite() {
            return Err(GisError::InvalidCoordinate(
                "UTM easting and northing must be finite".to_owned(),
            ));
        }
        Ok(Self {
            zone,
            hemisphere,
            easting,
            northing,
        })
    }

    /// Converts the UTM coordinate to a decimal latitude and longitude (WGS84) pair.
    pub fn to_lat_lon(&self) -> Result<(f64, f64), GisError> {
        let zone_letter = match self.hemisphere {
            CardinalDirection::North => 'N',
            CardinalDirection::South => 'M', // 'M' is Southern hemisphere band
            _ => {
                return Err(GisError::InvalidCoordinate(
                    "UTM hemisphere must be North or South".to_owned(),
                ));
            }
        };
        let (lat, lon) =
            utm::wsg84_utm_to_lat_lon(self.easting, self.northing, self.zone, zone_letter)
                .map_err(|error| GisError::Operation(format!("UTM conversion error: {error:?}")))?;
        Ok((lat, lon))
    }

    /// Converts a decimal latitude and longitude (WGS84) to a `UtmCoordinate`.
    pub fn from_lat_lon(latitude: f64, longitude: f64) -> Result<Self, GisError> {
        if !latitude.is_finite() || !(-80.0..=84.0).contains(&latitude) {
            return Err(GisError::InvalidCoordinate(
                "UTM coordinates are only defined between 80S and 84N".to_owned(),
            ));
        }
        if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
            return Err(GisError::InvalidCoordinate(
                "longitude must be finite and between -180 and 180".to_owned(),
            ));
        }
        let zone = (((longitude + 180.0) / 6.0).floor() as u8 + 1).min(60);
        let (northing, easting, _) = utm::to_utm_wgs84(latitude, longitude, zone);
        let hemisphere = if latitude >= 0.0 {
            CardinalDirection::North
        } else {
            CardinalDirection::South
        };
        Ok(Self {
            zone,
            hemisphere,
            easting,
            northing,
        })
    }
}

/// Entry point for automatic coordinate detection and conversion.
pub struct CoordinateConverter;

impl CoordinateConverter {
    /// Automatically detects the format of a coordinate string and parses it to decimal degrees.
    ///
    /// Supports:
    /// - Decimal Degrees (e.g. `41.403389`, `-123.45`, `41.403389 N`)
    /// - Degrees Decimal Minutes (e.g. `41° 24.2028' N`, `41 24.2028 N`)
    /// - Degrees Minutes Seconds (e.g. `41° 24' 12.2" N`, `41 24 12.2 N`)
    pub fn parse_to_decimal<S>(s: S) -> Result<f64, GisError>
    where
        S: AsRef<str>,
    {
        let text = s.as_ref().trim();
        let clean_upper = text.to_ascii_uppercase();
        let is_negative =
            clean_upper.contains('S') || clean_upper.contains('W') || text.contains('-');

        let tokens = tokenize_positive_numbers(text);

        let abs_val = match tokens.len() {
            1 => tokens[0],
            2 => {
                let ddm: DdmCoordinate = text.parse()?;
                ddm.to_decimal().abs()
            }
            3 => {
                let dms: DmsCoordinate = text.parse()?;
                dms.to_decimal().abs()
            }
            _ => {
                return Err(GisError::Data(format!(
                    "Unable to parse coordinate: '{}'. Expected 1, 2, or 3 numeric components.",
                    text
                )));
            }
        };

        if is_negative {
            Ok(-abs_val)
        } else {
            Ok(abs_val)
        }
    }
}

fn validate_degrees(
    degrees: u32,
    minutes: f64,
    seconds: f64,
    direction: CardinalDirection,
) -> Result<(), GisError> {
    let maximum = match direction {
        CardinalDirection::North | CardinalDirection::South => 90,
        CardinalDirection::East | CardinalDirection::West => 180,
    };
    if degrees > maximum || (degrees == maximum && (minutes > 0.0 || seconds > 0.0)) {
        return Err(GisError::InvalidCoordinate(format!(
            "degrees exceed the valid range for {direction:?}"
        )));
    }
    Ok(())
}

/// Helper to tokenize a string into positive floating point numbers.
fn tokenize_positive_numbers(s: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    let mut current = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            current.push(c);
        } else {
            if !current.is_empty() {
                if let Ok(val) = current.parse::<f64>() {
                    numbers.push(val);
                }
                current.clear();
            }
        }
    }
    if let Ok(val) = current.parse::<f64>() {
        numbers.push(val);
    }
    numbers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dms_conversions() {
        let coord = DmsCoordinate::new(41, 24, 12.2, CardinalDirection::North);
        let val = coord.to_decimal();
        assert!((val - 41.40338888888889).abs() < 1e-9);

        let coord_s = DmsCoordinate::new(41, 24, 12.2, CardinalDirection::South);
        assert!((coord_s.to_decimal() - (-41.40338888888889)).abs() < 1e-9);

        let parsed: DmsCoordinate = "41° 24' 12.2\" N".parse().unwrap();
        assert_eq!(parsed.degrees, 41);
        assert_eq!(parsed.minutes, 24);
        assert!((parsed.seconds - 12.2).abs() < 1e-9);
        assert_eq!(parsed.direction, CardinalDirection::North);
    }

    #[test]
    fn test_ddm_conversions() {
        let coord = DdmCoordinate::new(41, 24.2028, CardinalDirection::North);
        let val = coord.to_decimal();
        assert!((val - 41.40338).abs() < 1e-9);

        let parsed: DdmCoordinate = "41° 24.2028' N".parse().unwrap();
        assert_eq!(parsed.degrees, 41);
        assert!((parsed.minutes - 24.2028).abs() < 1e-9);
        assert_eq!(parsed.direction, CardinalDirection::North);
    }

    #[test]
    fn test_utm_conversions() {
        let utm_coord = UtmCoordinate::from_lat_lon(34.0522, -118.2437).unwrap();
        assert_eq!(utm_coord.zone, 11);
        assert_eq!(utm_coord.hemisphere, CardinalDirection::North);
        assert!((utm_coord.easting - 385153.0).abs() < 1000.0);
        assert!((utm_coord.northing - 3768853.0).abs() < 1000.0);

        let (lat, lon) = utm_coord.to_lat_lon().unwrap();
        assert!((lat - 34.0522).abs() < 1e-4);
        assert!((lon - (-118.2437)).abs() < 1e-4);

        let boundary = UtmCoordinate::from_lat_lon(0.0, 180.0)
            .expect("the antimeridian should have a valid UTM zone");
        assert_eq!(boundary.zone, 60);
        assert!(UtmCoordinate::from_lat_lon(0.0, 181.0).is_err());
    }

    #[test]
    fn rejects_invalid_sexagesimal_components() {
        assert!("41 60 0 N".parse::<DmsCoordinate>().is_err());
        assert!("41 0 60 N".parse::<DmsCoordinate>().is_err());
        assert!("91 0 N".parse::<DdmCoordinate>().is_err());
        assert!("181 0 E".parse::<DdmCoordinate>().is_err());
    }

    #[test]
    fn test_auto_detect_parser() {
        assert!(
            (CoordinateConverter::parse_to_decimal("41.403389").unwrap() - 41.403389).abs() < 1e-9
        );
        assert!(
            (CoordinateConverter::parse_to_decimal("-123.45").unwrap() - (-123.45)).abs() < 1e-9
        );

        assert!(
            (CoordinateConverter::parse_to_decimal("41 24.2028 N").unwrap() - 41.40338).abs()
                < 1e-9
        );
        assert!(
            (CoordinateConverter::parse_to_decimal("41° 24.2028' S").unwrap() - (-41.40338)).abs()
                < 1e-9
        );

        assert!(
            (CoordinateConverter::parse_to_decimal("41 24 12.2 N").unwrap() - 41.403388888).abs()
                < 1e-4
        );
        assert!(
            (CoordinateConverter::parse_to_decimal("-41 24 12.2").unwrap() - (-41.403388888)).abs()
                < 1e-4
        );
    }
}
