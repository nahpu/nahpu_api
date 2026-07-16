//! # Darwin Core Mapper
//!
//! This module contains the mapping logic from the Nahpu database schema
//! to the Darwin Core standard terms. This mapping is manually defined based on
//! the `tables_dwc_mapping.md` documentation.

/// A utility struct for mapping Nahpu schema names to Darwin Core terms.
pub struct DwcMapper;

/// Describes how one NAHPU source field is represented in a flat Darwin Core
/// export. A source can emit more than one column (for example a measurement
/// emits type, value, and unit) and visible headers are intentionally allowed
/// to repeat between source fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DwcMapping {
    pub headers: Vec<&'static str>,
    pub measurement_type: Option<&'static str>,
    pub measurement_unit: Option<&'static str>,
}

impl DwcMapper {
    // --- Public entry point ---
    /// Maps a table and column name from the Nahpu schema to the corresponding Darwin Core term.
    pub fn get_dwc_term(table_name: &str, column_name: &str) -> Option<&'static str> {
        match table_name {
            "project" => Self::map_project_column(column_name),
            "site" => Self::map_site_column(column_name),
            "coordinate" => Self::map_coordinate_column(column_name),
            "collEvent" => Self::map_coll_event_column(column_name),
            "collPersonnel" => Self::map_coll_personnel_column(column_name),
            "collEffort" => Self::map_coll_effort_column(column_name),
            "narrative" => Self::map_narrative_column(column_name),
            "media" => Self::map_media_column(column_name),
            "associatedData" => Self::map_associated_data_column(column_name),
            "personnel" => Self::map_personnel_column(column_name),
            "taxonomy" => Self::map_taxonomy_column(column_name),
            "specimen" => Self::map_specimen_column(column_name),
            "specimenPart" => Self::map_specimen_part_column(column_name),
            "narrativeMedia" => Self::map_narrative_media_column(column_name),
            "siteMedia" => Self::map_site_media_column(column_name),
            "specimenMedia" => Self::map_specimen_media_column(column_name),
            "personnelList" => Self::map_personnel_list_column(column_name),
            "weather" => Self::map_weather_column(column_name),
            "mammalMeasurement" => Self::map_mammal_measurement_column(column_name),
            "avianMeasurement" => Self::map_avian_measurement_column(column_name),
            _ => None,
        }
    }

    /// Resolves a NAHPU source key in `table::field` form to a Darwin Core term.
    ///
    /// This is the preferred entry point for flat exports because it keeps the
    /// source-key parsing and table aliases in the Darwin Core crate.
    pub fn get_dwc_term_for_source_key(source_key: &str) -> Option<&'static str> {
        let (table_name, column_name) = source_key.split_once("::")?;
        let table_name = match table_name {
            "event" => "collEvent",
            table_name => table_name,
        };
        Self::get_dwc_term(table_name, column_name)
    }

    /// Resolves a source key to its complete tabular Darwin Core mapping.
    ///
    /// Most sources emit a single direct term. Measurements deliberately emit
    /// a repeated MeasurementOrFact column group so that every selected NAHPU
    /// measurement retains its type and unit in a flat export.
    pub fn get_dwc_mapping_for_source_key(source_key: &str) -> Option<DwcMapping> {
        if let Some(mapping) = Self::measurement_mapping(source_key) {
            return Some(mapping);
        }

        match source_key {
            "coordinate::elevationInMeter" => Some(DwcMapping {
                headers: vec![
                    "dwc:minimumElevationInMeters",
                    "dwc:maximumElevationInMeters",
                ],
                measurement_type: None,
                measurement_unit: None,
            }),
            "specimenPart::type" => Some(DwcMapping {
                headers: vec!["dwc:materialEntityType", "dwc:objectQuantityType"],
                measurement_type: None,
                measurement_unit: None,
            }),
            "specimenPart::count" => Some(DwcMapping {
                headers: vec!["dwc:objectQuantity"],
                measurement_type: None,
                measurement_unit: None,
            }),
            _ => Self::get_dwc_term_for_source_key(source_key).map(|header| DwcMapping {
                headers: vec![header],
                measurement_type: None,
                measurement_unit: None,
            }),
        }
    }

    fn measurement_mapping(source_key: &str) -> Option<DwcMapping> {
        let (measurement_type, measurement_unit) = match source_key {
            "mammalMeasurement::totalLength" => ("total length", Some("mm")),
            "mammalMeasurement::tailLength" => ("tail length", Some("mm")),
            "mammalMeasurement::hindFootLength" => ("hind foot length", Some("mm")),
            "mammalMeasurement::earLength" => ("ear length", Some("mm")),
            "mammalMeasurement::forearm" => ("forearm length", Some("mm")),
            "mammalMeasurement::tibia" => ("tibia length", Some("mm")),
            "mammalMeasurement::weight" => ("weight", Some("g")),
            "mammalMeasurement::frequencyMax" => ("maximum frequency", Some("kHz")),
            "mammalMeasurement::frequencyMin" => ("minimum frequency", Some("kHz")),
            "mammalMeasurement::frequencyAtMaxEnergy" => {
                ("frequency at maximum energy", Some("kHz"))
            }
            "mammalMeasurement::duration" => ("echolocation duration", Some("s")),
            "mammalMeasurement::testisPosition" => ("testis position", None),
            "mammalMeasurement::testisLength" => ("testis length", Some("mm")),
            "mammalMeasurement::testisWidth" => ("testis width", Some("mm")),
            "mammalMeasurement::epididymisAppearance" => ("epididymis appearance", None),
            "mammalMeasurement::leftPlacentalScars" => ("left placental scars", None),
            "mammalMeasurement::rightPlacentalScars" => ("right placental scars", None),
            "mammalMeasurement::mammaeCondition" => ("mammae condition", None),
            "mammalMeasurement::mammaeInguinalCount" => ("inguinal mammae count", None),
            "mammalMeasurement::mammaeAxillaryCount" => ("axillary mammae count", None),
            "mammalMeasurement::mammaeAbdominalCount" => ("abdominal mammae count", None),
            "mammalMeasurement::vaginaOpening" => ("vagina opening", None),
            "mammalMeasurement::pubicSymphysis" => ("pubic symphysis", None),
            "mammalMeasurement::embryoLeftCount" => ("left embryo count", None),
            "mammalMeasurement::embryoRightCount" => ("right embryo count", None),
            "mammalMeasurement::embryoCR" => ("embryo crown-rump length", Some("mm")),
            "mammalMeasurement::echolocation" => ("echolocation", None),
            "avianMeasurement::weight" => ("weight", Some("g")),
            "avianMeasurement::wingspan" => ("wingspan", Some("mm")),
            "avianMeasurement::bursaWidth" => ("bursa width", Some("mm")),
            "avianMeasurement::bursaLength" => ("bursa length", Some("mm")),
            "avianMeasurement::testisLength" => ("testis length", Some("mm")),
            "avianMeasurement::testisWidth" => ("testis width", Some("mm")),
            "avianMeasurement::ovaryLength" => ("ovary length", Some("mm")),
            "avianMeasurement::ovaryWidth" => ("ovary width", Some("mm")),
            "avianMeasurement::oviductWidth" => ("oviduct width", Some("mm")),
            "avianMeasurement::firstOvaSize" => ("first ova size", Some("mm")),
            "avianMeasurement::secondOvaSize" => ("second ova size", Some("mm")),
            "avianMeasurement::thirdOvaSize" => ("third ova size", Some("mm")),
            "avianMeasurement::skullOssification" => ("skull ossification", Some("%")),
            "avianMeasurement::irisColor" => ("iris color", None),
            "avianMeasurement::irisHex" => ("iris color hex", None),
            "avianMeasurement::billColor" => ("bill color", None),
            "avianMeasurement::billHex" => ("bill color hex", None),
            "avianMeasurement::footColor" => ("foot color", None),
            "avianMeasurement::footHex" => ("foot color hex", None),
            "avianMeasurement::tarsusColor" => ("tarsus color", None),
            "avianMeasurement::tarsusHex" => ("tarsus color hex", None),
            "avianMeasurement::broodPatch" => ("brood patch", None),
            "avianMeasurement::hasBursa" => ("bursa present", None),
            "avianMeasurement::fat" => ("fat score", None),
            "avianMeasurement::stomachContent" => ("stomach content", None),
            "avianMeasurement::testisRemark" => ("testis remarks", None),
            "avianMeasurement::ovaryAppearance" => ("ovary appearance", None),
            "avianMeasurement::oviductAppearance" => ("oviduct appearance", None),
            "avianMeasurement::ovaryRemark" => ("ovary remarks", None),
            "avianMeasurement::wingIsMolt" => ("wing molt present", None),
            "avianMeasurement::wingMolt" => ("wing molt", None),
            "avianMeasurement::tailIsMolt" => ("tail molt present", None),
            "avianMeasurement::tailMolt" => ("tail molt", None),
            "avianMeasurement::bodyMolt" => ("body molt", None),
            "avianMeasurement::moltRemark" => ("molt remarks", None),
            "herpMeasurement::weight" => ("weight", Some("g")),
            "herpMeasurement::svl" => ("snout-vent length", Some("cm")),
            "weather::lowestDayTempC" => ("lowest day temperature", Some("°C")),
            "weather::highestDayTempC" => ("highest day temperature", Some("°C")),
            "weather::lowestNightTempC" => ("lowest night temperature", Some("°C")),
            "weather::highestNightTempC" => ("highest night temperature", Some("°C")),
            "weather::averageHumidity" => ("average humidity", Some("%")),
            "weather::dewPointTemp" => ("dew point temperature", Some("°C")),
            "weather::sunriseTime" => ("sunrise", Some("hh:mm:ss")),
            "weather::sunsetTime" => ("sunset", Some("hh:mm:ss")),
            "weather::moonPhase" => ("moon phase", None),
            _ => return None,
        };
        Some(DwcMapping {
            headers: vec![
                "dwc:measurementType",
                "dwc:measurementValue",
                "dwc:measurementUnit",
            ],
            measurement_type: Some(measurement_type),
            measurement_unit,
        })
    }

    fn map_project_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "uuid" => Some("dcterms:identifier"),
            "name" => Some("dwc:datasetName"),
            "startDate" | "endDate" => Some("dwc:eventDate"),
            "created" => Some("dcterms:created"),
            "lastAccessed" => Some("dcterms:modified"),
            _ => None,
        }
    }

    fn map_site_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "siteID" | "siteId" => Some("dwc:siteNumber"),
            "projectUuid" => Some("dwc:datasetID"),
            "siteType" => Some("dwc:locationRemarks"),
            "country" => Some("dwc:country"),
            "stateProvince" => Some("dwc:stateProvince"),
            "county" => Some("dwc:county"),
            "municipality" => Some("dwc:municipality"),
            "locality" => Some("dwc:verbatimLocality"),
            "remark" => Some("dwc:locationRemarks"),
            "habitatType" | "habitatCondition" | "habitatDescription" => Some("dwc:habitat"),
            _ => None,
        }
    }

    fn map_coordinate_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "siteID" | "siteId" => Some("dwc:locationID"),
            "decimalLatitude" => Some("dwc:decimalLatitude"),
            "decimalLongitude" => Some("dwc:decimalLongitude"),
            "elevationInMeter" => Some("dwc:minimumElevationInMeters"),
            "datum" => Some("dwc:geodeticDatum"),
            "uncertaintyInMeters" => Some("dwc:coordinateUncertaintyInMeters"),
            "notes" => Some("dwc:georeferenceRemarks"),
            _ => None,
        }
    }

    fn map_coll_event_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "id" => Some("dwc:eventID"),
            "projectUuid" => Some("dwc:datasetID"),
            "siteID" | "siteId" => Some("dwc:locationID"),
            "startDate" | "endDate" => Some("dwc:eventDate"),
            "startTime" | "endTime" => Some("dwc:eventTime"),
            "primaryCollMethod" => Some("dwc:samplingProtocol"),
            "collMethodNotes" => Some("dwc:samplingEffort"),
            "personnel" => Some("dwc:recordedBy"),
            _ => None,
        }
    }

    fn map_coll_personnel_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "eventID" | "eventId" => Some("dwc:eventID"),
            "personnelId" => Some("dwc:recordedByID"),
            "name" => Some("dwc:recordedBy"),
            _ => None,
        }
    }

    fn map_coll_effort_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "eventID" | "eventId" => Some("dwc:eventID"),
            "method" | "brand" => Some("dwc:samplingProtocol"),
            "notes" => Some("dwc:samplingEffort"),
            _ => None,
        }
    }

    fn map_narrative_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "projectUuid" => Some("dwc:datasetID"),
            "siteID" | "siteId" => Some("dwc:locationID"),
            "date" => Some("dcterms:date"),
            "narrative" => Some("dwc:eventRemarks"),
            _ => None,
        }
    }

    fn map_media_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "primaryId" | "secondaryId" => Some("dcterms:identifier"),
            "projectUuid" => Some("dwc:datasetID"),
            "category" => Some("dcterms:type"),
            "tag" => Some("dcterms:subject"),
            "taken" => Some("dcterms:created"),
            "camera" | "lenses" | "additionalExif" => Some("dcterms:description"),
            "personnelId" => Some("dcterms:creator"),
            "fileName" => Some("dcterms:title"),
            "caption" => Some("dcterms:description"),
            _ => None,
        }
    }

    fn map_associated_data_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "name" => Some("dcterms:title"),
            "type" => Some("dcterms:type"),
            "date" => Some("dcterms:created"),
            "description" => Some("dcterms:description"),
            "url" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_personnel_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "uuid" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_taxonomy_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "id" => Some("dwc:taxonID"),
            "taxonClass" => Some("dwc:class"),
            "taxonOrder" => Some("dwc:order"),
            "taxonFamily" => Some("dwc:family"),
            "genus" => Some("dwc:genus"),
            "specificEpithet" => Some("dwc:specificEpithet"),
            "authors" => Some("dwc:scientificNameAuthorship"),
            "commonName" => Some("dwc:vernacularName"),
            "notes" => Some("dwc:taxonRemarks"),
            _ => None,
        }
    }

    fn map_specimen_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "uuid" => Some("dwc:occurrenceID"),
            "projectUuid" => Some("dwc:datasetID"),
            "speciesID" | "speciesId" => Some("dwc:taxonID"),
            "scientificName" => Some("dwc:scientificName"),
            "iDMethod" => Some("dwc:identificationType"),
            "taxonGroup" => Some("dwc:higherClassification"),
            "collectionDate" | "captureDate" => Some("dwc:eventDate"),
            "collectionTime" | "captureTime" => Some("dwc:eventTime"),
            "trapType" | "methodID" | "methodId" | "collMethodID" | "collMethodId" => {
                Some("dwc:samplingProtocol")
            }
            "coordinateID" | "coordinateId" => Some("dwc:locationID"),
            "catalogerID" | "catalogerId" | "collPersonnelID" | "collPersonnelId" => {
                Some("dwc:recordedBy")
            }
            "fieldNumber" => Some("dwc:recordNumber"),
            "collEventID" | "collEventId" => Some("dwc:eventID"),
            "museumID" | "museumId" => Some("dwc:institutionCode"),
            "preparatorID" | "preparatorId" => Some("dwc:recordedBy"),
            _ => None,
        }
    }

    fn map_specimen_part_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "tissueID" | "tissueId" => Some("dwc:materialSampleID"),
            "barcodeID" | "barcodeId" => Some("dwc:otherCatalogNumbers"),
            "treatment" | "additionalTreatment" => Some("dwc:preparations"),
            "count" => Some("dwc:objectQuantity"),
            "dateTaken" => Some("dwc:eventDate"),
            "timeTaken" => Some("dwc:eventTime"),
            "remark" | "pmi" => Some("dwc:occurrenceRemarks"),
            _ => None,
        }
    }

    fn map_narrative_media_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "narrativeId" | "mediaId" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_site_media_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "siteId" => Some("dwc:locationID"),
            "mediaId" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_specimen_media_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "mediaId" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_personnel_list_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "projectUuid" => Some("dwc:datasetID"),
            "personnelUuid" => Some("dcterms:identifier"),
            _ => None,
        }
    }

    fn map_weather_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "eventID" | "eventId" => Some("dwc:eventID"),
            "notes" => Some("dwc:eventRemarks"),
            _ => None,
        }
    }

    fn map_mammal_measurement_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "sex" => Some("dwc:sex"),
            "age" => Some("dwc:lifeStage"),
            "reproductiveStage" => Some("dwc:reproductiveCondition"),
            "remark" => Some("dwc:occurrenceRemarks"),
            _ => None,
        }
    }

    fn map_avian_measurement_column(column_name: &str) -> Option<&'static str> {
        match column_name {
            "specimenUuid" => Some("dwc:occurrenceID"),
            "sex" => Some("dwc:sex"),
            "specimenRemark" => Some("dwc:occurrenceRemarks"),
            "habitatRemark" => Some("dwc:habitat"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DwcMapper;

    const CURRENT_DWC_TERMS_USED_BY_NAHPU: &[&str] = &[
        "dwc:class",
        "dwc:coordinateUncertaintyInMeters",
        "dwc:country",
        "dwc:county",
        "dwc:datasetID",
        "dwc:datasetName",
        "dwc:decimalLatitude",
        "dwc:decimalLongitude",
        "dwc:eventDate",
        "dwc:eventID",
        "dwc:eventRemarks",
        "dwc:eventTime",
        "dwc:family",
        "dwc:genus",
        "dwc:geodeticDatum",
        "dwc:georeferenceRemarks",
        "dwc:habitat",
        "dwc:higherClassification",
        "dwc:identificationType",
        "dwc:institutionCode",
        "dwc:lifeStage",
        "dwc:locationID",
        "dwc:locationRemarks",
        "dwc:materialSampleID",
        "dwc:measurementRemarks",
        "dwc:minimumElevationInMeters",
        "dwc:municipality",
        "dwc:occurrenceID",
        "dwc:occurrenceRemarks",
        "dwc:order",
        "dwc:otherCatalogNumbers",
        "dwc:objectQuantity",
        "dwc:preparations",
        "dwc:recordNumber",
        "dwc:recordedBy",
        "dwc:recordedByID",
        "dwc:reproductiveCondition",
        "dwc:samplingEffort",
        "dwc:samplingProtocol",
        "dwc:scientificNameAuthorship",
        "dwc:sex",
        "dwc:siteNumber",
        "dwc:specificEpithet",
        "dwc:stateProvince",
        "dwc:taxonID",
        "dwc:taxonRemarks",
        "dwc:verbatimLocality",
        "dwc:vernacularName",
    ];

    #[test]
    fn mapped_dwc_terms_match_the_current_official_term_names() {
        let source_keys = [
            "project::name",
            "project::startDate",
            "site::siteID",
            "site::projectUuid",
            "site::siteType",
            "site::country",
            "site::stateProvince",
            "site::county",
            "site::municipality",
            "site::locality",
            "site::remark",
            "site::habitatType",
            "coordinate::decimalLatitude",
            "coordinate::decimalLongitude",
            "coordinate::elevationInMeter",
            "coordinate::datum",
            "coordinate::uncertaintyInMeters",
            "collEvent::startDate",
            "collEvent::startTime",
            "collEvent::primaryCollMethod",
            "collEvent::collMethodNotes",
            "collPersonnel::name",
            "collPersonnel::personnelId",
            "collEffort::method",
            "collEffort::notes",
            "narrative::narrative",
            "taxonomy::id",
            "taxonomy::taxonClass",
            "taxonomy::taxonOrder",
            "taxonomy::taxonFamily",
            "taxonomy::genus",
            "taxonomy::specificEpithet",
            "taxonomy::authors",
            "taxonomy::commonName",
            "taxonomy::notes",
            "specimen::uuid",
            "specimen::speciesID",
            "specimen::iDMethod",
            "specimen::taxonGroup",
            "specimen::trapType",
            "specimen::coordinateID",
            "specimen::fieldNumber",
            "specimen::museumID",
            "specimen::preparatorID",
            "specimenPart::barcodeID",
            "specimenPart::tissueID",
            "specimenPart::count",
            "specimenPart::remark",
            "weather::notes",
            "mammalMeasurement::sex",
            "mammalMeasurement::age",
            "mammalMeasurement::reproductiveStage",
            "mammalMeasurement::remark",
            "avianMeasurement::habitatRemark",
            "avianMeasurement::specimenRemark",
        ];

        for source_key in source_keys {
            let Some(term) = DwcMapper::get_dwc_term_for_source_key(source_key) else {
                panic!("expected a mapping for {source_key}");
            };
            if term.starts_with("dwc:") {
                assert!(
                    CURRENT_DWC_TERMS_USED_BY_NAHPU.contains(&term),
                    "{term} is not an approved current Darwin Core term"
                );
            }
        }
    }

    #[test]
    fn unsupported_legacy_terms_are_not_emitted() {
        for source_key in [
            "project::description",
            "project::location",
            "taxonomy::citesStatus",
            "taxonomy::redListCategory",
            "taxonomy::countryStatus",
        ] {
            assert_eq!(DwcMapper::get_dwc_term_for_source_key(source_key), None);
        }
    }

    #[test]
    fn supports_schema_acronyms_and_legacy_aliases() {
        for (source_key, expected) in [
            ("site::siteID", "dwc:siteNumber"),
            ("coordinate::siteID", "dwc:locationID"),
            ("collPersonnel::eventID", "dwc:eventID"),
            ("collEffort::eventID", "dwc:eventID"),
            ("specimen::speciesID", "dwc:taxonID"),
            ("specimen::collEventID", "dwc:eventID"),
            ("specimenPart::tissueID", "dwc:materialSampleID"),
            ("site::siteId", "dwc:siteNumber"),
            ("specimen::speciesId", "dwc:taxonID"),
            ("specimenPart::tissueId", "dwc:materialSampleID"),
            ("event::id", "dwc:eventID"),
        ] {
            assert_eq!(
                DwcMapper::get_dwc_term_for_source_key(source_key),
                Some(expected)
            );
        }
    }

    #[test]
    fn measurement_sources_expand_to_unsuffixed_measurement_or_fact_columns() {
        let mapping = DwcMapper::get_dwc_mapping_for_source_key("mammalMeasurement::tailLength")
            .expect("tail length should be mapped");
        assert_eq!(
            mapping.headers,
            vec![
                "dwc:measurementType",
                "dwc:measurementValue",
                "dwc:measurementUnit",
            ]
        );
        assert_eq!(mapping.measurement_type, Some("tail length"));
        assert_eq!(mapping.measurement_unit, Some("mm"));
    }

    #[test]
    fn preparator_is_an_agent_not_a_preparation_method() {
        assert_eq!(
            DwcMapper::get_dwc_term_for_source_key("specimen::preparatorID"),
            Some("dwc:recordedBy")
        );
        assert_eq!(
            DwcMapper::get_dwc_term_for_source_key("specimenPart::treatment"),
            Some("dwc:preparations")
        );
    }
}
