use crate::dwc::DwcMapper;
use serde_json::{Map, Value};
use std::collections::HashMap;

pub mod json;
pub mod xml;

pub(crate) const NAHPU_XML_NAMESPACE: &str = "https://www.nahpu.app/terms/";

/// Maps one serialized NAHPU row without discarding values.
///
/// A standard term is used only when exactly one populated source field maps
/// to it. Unmapped and colliding fields keep their table and column identity
/// in the NAHPU extension namespace.
pub(crate) fn map_serialized_fields(table_name: &str, value: Value) -> Vec<(String, Value)> {
    let Value::Object(fields) = value else {
        return Vec::new();
    };
    let fields = populated_fields(fields);
    let mut target_counts = HashMap::<&str, usize>::new();
    for source_field in fields.keys() {
        if let Some(target) = DwcMapper::get_dwc_term(table_name, source_field) {
            *target_counts.entry(target).or_default() += 1;
        }
    }

    fields
        .into_iter()
        .map(|(source_field, value)| {
            let mapped = DwcMapper::get_dwc_term(table_name, &source_field)
                .filter(|target| target_counts.get(target) == Some(&1));
            let output_field = mapped.map_or_else(
                || format!("nahpu:{table_name}.{source_field}"),
                str::to_string,
            );
            (output_field, value)
        })
        .collect()
}

fn populated_fields(fields: Map<String, Value>) -> Map<String, Value> {
    fields
        .into_iter()
        .filter(|(_, value)| {
            !value.is_null() && !matches!(value, Value::String(text) if text.is_empty())
        })
        .collect()
}
