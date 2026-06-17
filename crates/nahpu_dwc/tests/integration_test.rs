use nahpu_dwc::export::json::convert_to_dwc_json;
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DummySite {
    site_id: String,
    country: String,
}

#[test]
fn test_site_dwc_conversion() {
    let site = DummySite {
        site_id: "S1".to_string(),
        country: "USA".to_string(),
    };

    let result = convert_to_dwc_json("site", &site).unwrap();
    assert_eq!(result["dwc:locationID"], json!("S1"));
    assert_eq!(result["dwc:country"], json!("USA"));
}
