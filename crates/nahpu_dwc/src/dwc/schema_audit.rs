use super::DwcMapper;
use sqlparser::ast::{ObjectNamePart, Statement};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::BTreeSet;

const SCHEMA: &str = include_str!("../../../nahpu_db/schema/tables.drift");

const DYNAMIC_TABLES: &[&str] = &["customFieldDefinition", "customFieldValue"];

const RELATIONSHIP_FIELDS: &[&str] = &[
    "collPersonnel::eventID",
    "collPersonnel::personnelId",
    "collPersonnel::role",
    "eventAssociatedData::associatedDataId",
    "eventAssociatedData::eventID",
    "eventMedia::eventID",
    "eventMedia::mediaId",
    "narrativeMedia::mediaId",
    "narrativeMedia::narrativeId",
    "parasite::anatomicalLocation",
    "parasite::associationStatus",
    "parasite::category",
    "parasite::specimenUuid",
    "personnelList::personnelUuid",
    "personnelList::projectUuid",
    "siteAssociatedData::associatedDataId",
    "siteAssociatedData::siteId",
    "site::geographyId",
    "siteMedia::mediaId",
    "siteMedia::siteId",
    "specimenAssociatedData::associatedDataId",
    "specimenAssociatedData::specimenUuid",
    "specimenPart::personnelId",
    "specimenPart::specimenUuid",
    "specimenMedia::mediaId",
    "specimenMedia::specimenUuid",
    "specimen::catalogerID",
    "specimen::preparatorID",
];

const COMPOSITE_SUPPORT_FIELDS: &[&str] = &[
    "birdAttribute::weightUnit",
    "fossilAttribute::weightUnit",
    "herpAttribute::weightUnit",
    "mammalAttribute::weightUnit",
    "specimen::condition",
    "specimen::coordinateExtentMeters",
    "specimen::projectFieldNumber",
];

const UNMAPPED_FIELDS: &[&str] = &[
    "collEffort::brand",
    "collEffort::count",
    "collEffort::id",
    "collEffort::size",
    "collEvent::idSuffix",
    "collPersonnel::id",
    "coordinate::gpsUnit",
    "coordinate::id",
    "fossilSite::biozone",
    "fossilSite::depositionalContinent",
    "fossilSite::depositionalEnvironmentType",
    "fossilSite::depositionalMarine",
    "fossilSite::geologicEpoch",
    "fossilSite::geologicEra",
    "fossilSite::geologicPeriod",
    "fossilSite::geologicSeries",
    "fossilSite::rockType",
    "fossilSite::sedimentologyRemark",
    "fossilSite::standardPreservationType",
    "fossilSite::stratigraphicSource",
    "fossilSite::stratigraphyRemark",
    "geography::id",
    "geography::matchKey",
    "mammalAttribute::accuracy",
    "mammalAttribute::accuracySpecify",
    "mammalAttribute::showBatFields",
    "mammalAttribute::showEchoFields",
    "narrative::id",
    "narrative::mediaID",
    "narrative::narrative",
    "narrative::time",
    "narrative::writerId",
    "parasite::datePreserved",
    "parasite::id",
    "parasite::museumLoan",
    "parasite::museumPermanent",
    "parasite::storage",
    "parasite::storageLocation",
    "parasite::timePreserved",
    "personnel::affiliation",
    "personnel::currentFieldNumber",
    "personnel::email",
    "personnel::initial",
    "personnel::isRegisterField",
    "personnel::phone",
    "personnel::photoPath",
    "personnel::role",
    "project::accession",
    "project::catalogNumberPrefix",
    "project::catalogNumberSuffix",
    "project::currentCatalogNumber",
    "project::description",
    "project::endDate",
    "project::location",
    "project::principalInvestigator",
    "project::startDate",
    "project::timeZone",
    "site::id",
    "site::leadStaffId",
    "site::mediaID",
    "site::siteType",
    "specimen::isMultipleCollector",
    "specimen::isRelativeTime",
    "specimen::museumID",
    "specimen::prepDate",
    "specimen::prepTime",
    "specimen::relativeCaptureTime",
    "specimenPart::id",
    "specimenPart::museumLoan",
    "specimenPart::museumPermanent",
    "specimenPart::pmi",
    "specimenPart::storage",
    "specimenPart::storageLocation",
    "taxonomy::citesStatus",
    "taxonomy::countryStatus",
    "taxonomy::mediaId",
    "taxonomy::redListCategory",
    "taxonomy::sortingOrder",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MappingStatus {
    Mapped,
    CompositeOrMeasurement,
    Relationship,
    Dynamic,
    Unmapped,
}

#[test]
fn every_schema_v21_field_has_an_explicit_darwin_core_status() {
    let schema_fields = schema_fields();
    let mut unclassified = Vec::new();

    for source in &schema_fields {
        if status_for(source).is_none() {
            unclassified.push(source.clone());
        }
    }

    assert!(
        unclassified.is_empty(),
        "schema fields need a Darwin Core classification:\n{}",
        unclassified.join("\n"),
    );

    for classified in RELATIONSHIP_FIELDS
        .iter()
        .chain(COMPOSITE_SUPPORT_FIELDS)
        .chain(UNMAPPED_FIELDS)
    {
        assert!(
            schema_fields.contains(*classified),
            "classification references missing schema field {classified}",
        );
    }
}

fn status_for(source: &str) -> Option<MappingStatus> {
    let (table, _) = source.split_once("::")?;
    if DYNAMIC_TABLES.contains(&table) {
        return Some(MappingStatus::Dynamic);
    }
    if RELATIONSHIP_FIELDS.contains(&source) {
        return Some(MappingStatus::Relationship);
    }
    if COMPOSITE_SUPPORT_FIELDS.contains(&source) {
        return Some(MappingStatus::CompositeOrMeasurement);
    }
    if let Some(mapping) = DwcMapper::get_dwc_mapping_for_source_key(source) {
        return Some(
            if mapping.headers.len() > 1 || mapping.measurement_type.is_some() {
                MappingStatus::CompositeOrMeasurement
            } else {
                MappingStatus::Mapped
            },
        );
    }
    UNMAPPED_FIELDS
        .contains(&source)
        .then_some(MappingStatus::Unmapped)
}

fn schema_fields() -> BTreeSet<String> {
    let dialect = GenericDialect {};
    let statements = Parser::parse_sql(&dialect, &create_table_statements(SCHEMA))
        .expect("schema v21 should parse as SQL");
    let mut fields = BTreeSet::new();

    for statement in statements {
        let Statement::CreateTable(table) = statement else {
            continue;
        };
        let Some(ObjectNamePart::Identifier(name)) = table.name.0.last() else {
            panic!("schema table should have an identifier");
        };
        for column in table.columns {
            fields.insert(format!("{}::{}", name.value, column.name.value));
        }
    }
    fields
}

fn create_table_statements(schema: &str) -> String {
    strip_sql_comments(schema)
        .split(';')
        .filter(|statement| {
            statement
                .trim()
                .to_ascii_uppercase()
                .starts_with("CREATE TABLE")
        })
        .map(|statement| format!("{};", statement.trim()))
        .collect()
}

fn strip_sql_comments(schema: &str) -> String {
    let mut cleaned = String::with_capacity(schema.len());
    let mut in_block_comment = false;

    for line in schema.lines() {
        let mut remainder = line;
        loop {
            if in_block_comment {
                let Some(end) = remainder.find("*/") else {
                    break;
                };
                remainder = &remainder[end + 2..];
                in_block_comment = false;
            } else if let Some(start) = remainder.find("/*") {
                cleaned.push_str(&remainder[..start]);
                remainder = &remainder[start + 2..];
                in_block_comment = true;
            } else {
                let code = remainder
                    .split_once("--")
                    .map_or(remainder, |(code, _)| code);
                cleaned.push_str(code);
                break;
            }
        }
        cleaned.push('\n');
    }
    cleaned
}
