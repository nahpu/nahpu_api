use serde::Serialize;
use serde_json::{Map, Value};
use crate::dwc::DwcMapper;

/// Converts a Nahpu database struct into a Darwin Core JSON object.
///
/// It iterates over the properties of the struct and maps the property names
/// to their equivalent Darwin Core terms using `DwcMapper`.
///
/// # Arguments
///
/// * `table_name` - The name of the table in the Nahpu schema (e.g. "project", "site")
/// * `record` - The serializable struct instance to be converted
///
/// # Returns
///
/// A `serde_json::Value` containing the converted Darwin Core JSON object.
pub fn convert_to_dwc_json<T: Serialize>(table_name: &str, record: &T) -> Result<Value, serde_json::Error> {
    let mut mapped_record = Map::new();
    let value = serde_json::to_value(record)?;
    
    if let Value::Object(map) = value {
        for (key, val) in map {
            // We can optionally filter out null values if needed for DWC export
            if val.is_null() { continue; } 
            
            // Map the property key to a DWC term, fallback to original key if not mapped
            let dwc_key = DwcMapper::get_dwc_term(table_name, &key).unwrap_or(&key);
            mapped_record.insert(dwc_key.to_string(), val);
        }
    }
    
    Ok(Value::Object(mapped_record))
}
