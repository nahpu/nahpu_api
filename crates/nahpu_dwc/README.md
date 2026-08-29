# nahpu_dwc

A utility crate for mapping and converting Nahpu project data into the Darwin Core (DwC) JSON format. It automatically pulls the Nahpu drift database schema and maps its struct fields natively into the `dwc` and `dcterms` namespaces.

## Example Usage

```rust
use nahpu_dwc::export::json::convert_to_dwc_json;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DummySite {
    site_id: String,
    country: String,
}

fn main() {
    let site = DummySite {
        site_id: "S1".to_string(),
        country: "USA".to_string(),
    };

    // Convert the struct to a Darwin Core mapped JSON
    let result = convert_to_dwc_json("site", &site).unwrap();
    
    // Output: {"dwc:locationID": "S1", "dwc:country": "USA"}
    println!("{}", result);
}
```

### Darwin Core specimen bundles

`package` creates the two NAHPU bundle formats from a JSON specimen snapshot:

- `DarwinCoreArchive` writes an occurrence-core DwC-A ZIP with `meta.xml`,
  `eml.xml`, optional extensions, and bundled media.
- `DarwinCoreDataPackage` writes a relational DwC Data Package as either
  tar.gz or ZIP. The archive contains `datapackage.json`, inline resource
  schemas, primary/foreign keys, EML, and available media at archive root.

TAR.GZ is the default DwC-DP container. ZIP is available as a compatibility
option and is reported in the returned manifest.

The package writer removes optional columns with no values and returns the same
deterministic manifest used by the NAHPU Bundle Project screen.

### Simple Darwin Core XML Export

To natively export an array of structs into the flat XML format compliant with the [Simple Darwin Core specification](https://dwc.tdwg.org/xml/), you can use `export_to_dwc_xml`:

```rust
use nahpu_dwc::export::xml::export_to_dwc_xml;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DummySite {
    site_id: String,
    country: String,
}

fn main() {
    let sites = vec![
        DummySite { site_id: "S1".to_string(), country: "USA".to_string() }
    ];

    let xml = export_to_dwc_xml("site", &sites).unwrap();
    println!("{}", xml);
}
```

## Darwin Core mappings

The complete NAHPU persistence model and the authoritative NAHPU-to-Darwin
Core field mapping are maintained in the [Persistence data documentation](https://nahpu.app/en/contributing/code/database/).

Version 0.5 maps only fields with an exact semantic equivalent. Unmapped
fields use `nahpu:<table>.<column>`. The generic JSON and XML converters use
that fallback directly; if populated source fields would collide on one
standard term, each value stays under its NAHPU key instead of being
overwritten. Weight measurements expose a `measurement_unit_source` so each
record can preserve its selected `g`, `kg`, or `lbs` unit. Verbatim coordinate
fields and determiner identifiers use their current Darwin Core terms.
The audit follows the normative
[Darwin Core List of Terms](https://dwc.tdwg.org/list/); terms that do not exist
there are not generated.

Update that page when the mapper or bundle output changes so the API
implementation and contributor documentation remain aligned.
