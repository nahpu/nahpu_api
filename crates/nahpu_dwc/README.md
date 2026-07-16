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

## Nahpu to Darwin Core Mappings

The following table summarizes how Nahpu database fields map to official Darwin Core terms.

| Nahpu Table | Nahpu Field | Darwin Core Term |
|---|---|---|
| **Project** | `uuid` | `dcterms:identifier` |
| | `name` | `dwc:datasetName` |
| | `principalInvestigator` | `dwc:recordedBy` |
| | `startDate`, `endDate` | `dwc:eventDate` |
| | `created` | `dcterms:created` |
| | `lastAccessed` | `dcterms:modified` |
| **Site** | `siteId` | `dwc:locationID` |
| | `projectUuid` | `dwc:datasetID` |
| | `leadStaffId` | `dwc:recordedBy` |
| | `siteType` | `dwc:locationRemarks` |
| | `country` | `dwc:country` |
| | `stateProvince` | `dwc:stateProvince` |
| | `county` | `dwc:county` |
| | `municipality` | `dwc:municipality` |
| | `locality` | `dwc:verbatimLocality` |
| | `remark` | `dwc:locationRemarks` |
| | `habitatType`, `habitatCondition`, `habitatDescription` | `dwc:habitat` |
| **Coordinate** | `siteId` | `dwc:locationID` |
| | `decimalLatitude` | `dwc:decimalLatitude` |
| | `decimalLongitude` | `dwc:decimalLongitude` |
| | `elevationInMeter` | `dwc:minimumElevationInMeters` |
| | `datum` | `dwc:geodeticDatum` |
| | `uncertaintyInMeters` | `dwc:coordinateUncertaintyInMeters` |
| | `gpsUnit`, `notes` | `dwc:georeferenceRemarks` |
| **CollEvent** | `id` | `dwc:eventID` |
| | `projectUuid` | `dwc:datasetID` |
| | `siteId` | `dwc:locationID` |
| | `startDate`, `endDate` | `dwc:eventDate` |
| | `startTime`, `endTime` | `dwc:eventTime` |
| | `primaryCollMethod` | `dwc:samplingProtocol` |
| | `collMethodNotes` | `dwc:samplingEffort` |
| **CollPersonnel** | `eventId` | `dwc:eventID` |
| | `personnelId` | `dwc:recordedByID` |
| | `name` | `dwc:recordedBy` |
| **CollEffort** | `eventId` | `dwc:eventID` |
| | `method`, `brand` | `dwc:samplingProtocol` |
| | `count`, `size` | `dwc:sampleSizeValue` |
| | `notes` | `dwc:samplingEffort` |
| **Taxonomy** | `id` | `dwc:taxonID` |
| | `taxonClass` | `dwc:class` |
| | `taxonOrder` | `dwc:order` |
| | `taxonFamily` | `dwc:family` |
| | `genus` | `dwc:genus` |
| | `specificEpithet` | `dwc:specificEpithet` |
| | `authors` | `dwc:scientificNameAuthorship` |
| | `commonName` | `dwc:vernacularName` |
| | `notes` | `dwc:taxonRemarks` |
| **Specimen** | `uuid` | `dwc:occurrenceID` |
| | `projectUuid` | `dwc:datasetID` |
| | `speciesId` | `dwc:taxonID` |
| | `iDConfidence` | `dwc:identificationQualifier` |
| | `iDMethod` | `dwc:identificationRemarks` |
| | `taxonGroup` | `dwc:higherClassification` |
| | `condition` | `dwc:disposition` |
| | `prepDate`, `prepTime` | `dcterms:modified` |
| | `collectionDate`, `captureDate` | `dwc:eventDate` |
| | `collectionTime`, `captureTime` | `dwc:eventTime` |
| | `trapType`, `methodId`, `collMethodId` | `dwc:samplingProtocol` |
| | `coordinateId` | `dwc:locationID` |
| | `catalogerId`, `collPersonnelId` | `dwc:recordedBy` |
| | `fieldNumber` | `dwc:recordNumber` |
| | `collEventId` | `dwc:eventID` |
| | `museumId` | `dwc:institutionCode` |
| | `preparatorId` | `dwc:preparations` |
| **SpecimenPart** | `specimenUuid` | `dwc:occurrenceID` |
| | `personnelId` | `dwc:recordedBy` |
| | `tissueId` | `dwc:materialSampleID` |
| | `barcodeId` | `dwc:otherCatalogNumbers` |
| | `type`, `treatment`, `additionalTreatment` | `dwc:preparations` |
| | `count` | `dwc:individualCount` |
| | `dateTaken` | `dwc:eventDate` |
| | `timeTaken` | `dwc:eventTime` |
| | `museumPermanent`, `museumLoan` | `dwc:disposition` |
| | `remark`, `pmi` | `dwc:occurrenceRemarks` |
| **Media** | `primaryId`, `secondaryId` | `dcterms:identifier` |
| | `projectUuid` | `dwc:datasetID` |
| | `category` | `dcterms:type` |
| | `tag` | `dcterms:subject` |
| | `taken` | `dcterms:created` |
| | `camera`, `lenses`, `additionalExif` | `dcterms:description` |
| | `personnelId` | `dcterms:creator` |
| | `fileName` | `dcterms:title` |
| | `caption` | `dcterms:description` |
