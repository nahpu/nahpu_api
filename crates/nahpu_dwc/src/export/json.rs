use super::map_serialized_fields;
use serde::Serialize;
use serde_json::{Map, Value};

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
pub fn convert_to_dwc_json<T: Serialize>(
    table_name: &str,
    record: &T,
) -> Result<Value, serde_json::Error> {
    let mut mapped_record = Map::new();
    for (field, value) in map_serialized_fields(table_name, serde_json::to_value(record)?) {
        mapped_record.insert(field, value);
    }

    Ok(Value::Object(mapped_record))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde_json::json;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct HabitatFields {
        habitat_type: String,
        habitat_condition: String,
        unmapped_field: String,
    }

    #[test]
    fn preserves_unmapped_and_colliding_fields_in_the_nahpu_namespace() {
        let result = convert_to_dwc_json(
            "siteAttribute",
            &HabitatFields {
                habitat_type: "forest".to_string(),
                habitat_condition: "disturbed".to_string(),
                unmapped_field: "kept".to_string(),
            },
        )
        .expect("JSON conversion should succeed");

        assert!(result.get("dwc:habitat").is_none());
        assert_eq!(result["nahpu:siteAttribute.habitatType"], json!("forest"));
        assert_eq!(
            result["nahpu:siteAttribute.habitatCondition"],
            json!("disturbed")
        );
        assert_eq!(result["nahpu:siteAttribute.unmappedField"], json!("kept"));
    }
}
