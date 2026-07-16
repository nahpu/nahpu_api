use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use serde::Serialize;
use serde_json::Value;
use std::io::Cursor;

/// Exports an array of struct records to the Simple Darwin Core XML format.
///
/// Automatically serializes the structs to determine their fields,
/// maps those fields to official Darwin Core terms, and generates an XML
/// structure fully conforming to the Simple Darwin Core standard (https://dwc.tdwg.org/xml/).
pub fn export_to_dwc_xml<T: Serialize>(
    table_name: &str,
    records: &[T],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let mut root = BytesStart::new("dwr:SimpleDarwinRecordSet");
    root.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
    root.push_attribute(("xsi:schemaLocation", "http://rs.tdwg.org/dwc/xsd/simpledarwincore/ http://rs.tdwg.org/dwc/xsd/tdwg_dwc_simple.xsd"));
    root.push_attribute(("xmlns:dcterms", "http://purl.org/dc/terms/"));
    root.push_attribute(("xmlns:dwc", "http://rs.tdwg.org/dwc/terms/"));
    root.push_attribute(("xmlns:dwr", "http://rs.tdwg.org/dwc/xsd/simpledarwincore/"));

    writer.write_event(Event::Start(root))?;

    for record in records {
        let val = serde_json::to_value(record)?;
        if let Value::Object(map) = val {
            writer.write_event(Event::Start(BytesStart::new("dwr:SimpleDarwinRecord")))?;

            for (key, field_val) in map {
                // Ignore Null values to keep XML clean
                if field_val.is_null() {
                    continue;
                }

                let str_val = match field_val {
                    Value::String(s) => s,
                    _ => field_val.to_string(), // basic serialization for numbers/booleans
                };

                if str_val.is_empty() {
                    continue;
                }

                let dwc_term =
                    crate::dwc::DwcMapper::get_dwc_term(table_name, &key).unwrap_or(&key);

                let field_tag = BytesStart::new(dwc_term);
                writer.write_event(Event::Start(field_tag.clone()))?;
                writer.write_event(Event::Text(BytesText::new(&str_val)))?;
                writer.write_event(Event::End(field_tag.to_end()))?;
            }

            writer.write_event(Event::End(BytesEnd::new("dwr:SimpleDarwinRecord")))?;
        }
    }

    writer.write_event(Event::End(BytesEnd::new("dwr:SimpleDarwinRecordSet")))?;

    let result = writer.into_inner().into_inner();
    let xml_string = String::from_utf8(result)?;
    Ok(xml_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DummySite {
        site_id: String,
        country: String,
    }

    #[test]
    fn test_xml_generation() {
        let sites = vec![
            DummySite {
                site_id: "S1".to_string(),
                country: "USA".to_string(),
            },
            DummySite {
                site_id: "S2".to_string(),
                country: "Canada & More".to_string(), // Test XML escaping
            },
        ];

        let xml = export_to_dwc_xml("site", &sites).unwrap();

        // Check XML declaration
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));

        // Check root and namespaces
        assert!(xml.contains("<dwr:SimpleDarwinRecordSet"));
        assert!(xml.contains("xmlns:dwc=\"http://rs.tdwg.org/dwc/terms/\""));

        // Check Record 1
        assert!(xml.contains("<dwr:SimpleDarwinRecord>"));
        assert!(xml.contains("<dwc:siteNumber>S1</dwc:siteNumber>"));
        assert!(xml.contains("<dwc:country>USA</dwc:country>"));

        // Check Record 2 & escaping
        assert!(xml.contains("<dwc:siteNumber>S2</dwc:siteNumber>"));
        // quick-xml encodes '&' as '&amp;'
        assert!(xml.contains("<dwc:country>Canada &amp; More</dwc:country>"));

        // Check closed elements
        assert!(xml.contains("</dwr:SimpleDarwinRecord>"));
        assert!(xml.ends_with("</dwr:SimpleDarwinRecordSet>"));
    }
}
