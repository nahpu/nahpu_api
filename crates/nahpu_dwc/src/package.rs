//! Darwin Core Archive and Darwin Core Data Package writers.
//!
//! The input is deliberately a small, transport-safe JSON snapshot. Database access
//! remains in NAHPU applications while Darwin Core semantics live in this crate.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use nahpu_archive::{
    tar_gzip::{TarGzipArchive, TarGzipExtractor},
    zip::{ZipArchive, ZipExtractor},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::dwc::terms::{BundleProfile, BundleTerm, TermRegistry};

const DWC_DP_PROFILE: &str = "http://rs.tdwg.org/dwc-dp/1.0/dwc-dp-profile.json";

/// Occurrence values the Data Package carries on its `identification` table instead.
const IDENTIFICATION_ONLY_FIELDS: &[&str] = &[
    "class",
    "family",
    "genus",
    "identificationType",
    "infraspecificEpithet",
    "kingdom",
    "order",
    "phylum",
    "specificEpithet",
    "taxonRemarks",
];

/// Determination values the Data Package repeats on `occurrence` and `identification`.
const IDENTIFICATION_SHARED_FIELDS: &[&str] = &[
    "identificationVerificationStatus",
    "identifiedBy",
    "identifiedByID",
    "scientificName",
    "scientificNameAuthorship",
    "taxonID",
    "taxonRank",
    "vernacularName",
];

/// Occurrence values the Data Package carries on its `event` table instead.
const EVENT_LOCATION_FIELDS: &[&str] = &[
    "coordinateUncertaintyInMeters",
    "decimalLatitude",
    "decimalLongitude",
    "geodeticDatum",
    "georeferenceRemarks",
    "locationRemarks",
    "maximumElevationInMeters",
    "minimumElevationInMeters",
    "verbatimCoordinateSystem",
    "verbatimCoordinates",
    "verbatimLatitude",
    "verbatimLongitude",
];

/// Occurrence values the Data Package already represents in another table or class.
const OCCURRENCE_FIELDS_REPRESENTED_ELSEWHERE: &[&str] = &[
    "associatedOccurrences",
    "basisOfRecord",
    "country",
    "county",
    "habitat",
    "islandGroup",
    "locality",
    "municipality",
    "recordedBy",
    "recordedByID",
    "stateProvince",
];

/// Specimen values the Data Package keeps as assertions when no material row carries them.
const OCCURRENCE_FIELDS_KEPT_AS_ASSERTIONS: &[(&str, &str)] = &[
    ("catalogNumber", "catalog number"),
    ("eventDate", "collection date"),
    ("eventTime", "collection time"),
    ("otherCatalogNumbers", "other catalog numbers"),
    ("preparations", "preparations"),
    ("samplingProtocol", "sampling protocol"),
];

/// Event values the Data Package keeps as assertions rather than as event columns.
const EVENT_FIELDS_KEPT_AS_ASSERTIONS: &[(&str, &str)] = &[
    ("samplingEffort", "sampling effort"),
    ("samplingProtocol", "sampling protocol"),
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BundleFormat {
    DarwinCoreArchive,
    DarwinCoreDataPackage,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat {
    TarGzip,
    Zip,
}

fn default_archive_format() -> ArchiveFormat {
    ArchiveFormat::TarGzip
}

fn effective_archive_format(request: &BundleRequest) -> ArchiveFormat {
    match request.format {
        BundleFormat::DarwinCoreArchive => ArchiveFormat::Zip,
        BundleFormat::DarwinCoreDataPackage => request.archive_format.clone(),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BundleRequest {
    pub format: BundleFormat,
    #[serde(default = "default_archive_format")]
    pub archive_format: ArchiveFormat,
    pub name: String,
    #[serde(default)]
    pub project: BTreeMap<String, Value>,
    #[serde(default)]
    pub occurrences: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub events: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub materials: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub measurements: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub occurrence_assertions: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub event_assertions: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub material_assertions: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub organism_interactions: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub organism_interaction_assertions: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub media: Vec<BtreeMedia>,
    #[serde(default)]
    pub agents: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub occurrence_agent_roles: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub event_agent_roles: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub material_agent_roles: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub media_agent_roles: Vec<BTreeMap<String, Value>>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// A media row and an optional local path to include in the package.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BtreeMedia {
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
    #[serde(default)]
    pub source_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BundleFile {
    pub path: String,
    pub media_type: String,
    pub records: usize,
    pub columns: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BundleManifest {
    pub format: BundleFormat,
    pub archive_format: ArchiveFormat,
    pub files: Vec<BundleFile>,
    pub warnings: Vec<String>,
}

/// One registered column of a bundle table, with the term URI it is published under.
#[derive(Clone, Debug)]
struct Column {
    header: String,
    term_uri: String,
    integer: bool,
}

impl Column {
    fn new(term: &BundleTerm) -> Self {
        Self {
            header: term.header.to_string(),
            term_uri: TermRegistry::term_uri(term),
            integer: term.integer,
        }
    }
}

#[derive(Clone, Debug)]
struct Table {
    name: &'static str,
    row_type: &'static str,
    core: bool,
    columns: Vec<Column>,
    rows: Vec<BTreeMap<String, String>>,
}

impl Table {
    fn headers(&self) -> Vec<&str> {
        self.columns
            .iter()
            .map(|column| column.header.as_str())
            .collect()
    }
}

/// Returns a deterministic file manifest without creating a bundle.
pub fn plan_bundle_json(input_json: &str) -> Result<String, String> {
    let request: BundleRequest =
        serde_json::from_str(input_json).map_err(|error| error.to_string())?;
    let manifest = build_manifest(&request)?;
    serde_json::to_string(&manifest).map_err(|error| error.to_string())
}

/// Writes a standards-shaped Darwin Core bundle.
///
/// Archives are always ZIP files; Data Packages are gzipped tarballs unless the request
/// asks for ZIP.
pub fn write_bundle_json(input_json: &str, output_path: &str) -> Result<String, String> {
    let request: BundleRequest =
        serde_json::from_str(input_json).map_err(|error| error.to_string())?;
    let manifest = write_bundle(&request, Path::new(output_path))?;
    serde_json::to_string(&manifest).map_err(|error| error.to_string())
}

/// Validates the package shape that NAHPU writes. It is intentionally structural,
/// so callers can show errors before sharing a bundle.
pub fn validate_bundle_json(input_json: &str) -> Result<String, String> {
    let request: BundleRequest =
        serde_json::from_str(input_json).map_err(|error| error.to_string())?;
    let mut errors = Vec::new();
    if request.occurrences.is_empty() {
        errors.push("A Darwin Core specimen bundle requires at least one occurrence.".to_string());
    }
    if request
        .occurrences
        .iter()
        .any(|row| !has_value(row, "occurrenceID"))
    {
        errors.push("Every occurrence requires an occurrenceID.".to_string());
    }
    let built = build_tables(&request);
    errors.extend(validate_relationships(&built.tables));
    if errors.is_empty() {
        Ok("[]".to_string())
    } else {
        serde_json::to_string(&errors).map_err(|error| error.to_string())
    }
}

fn write_bundle(request: &BundleRequest, output_path: &Path) -> Result<BundleManifest, String> {
    let manifest = build_manifest(request)?;
    match request.format {
        BundleFormat::DarwinCoreArchive => write_archive(request, output_path, &manifest)?,
        BundleFormat::DarwinCoreDataPackage => write_data_package(request, output_path, &manifest)?,
    }
    Ok(manifest)
}

fn build_manifest(request: &BundleRequest) -> Result<BundleManifest, String> {
    if request.occurrences.is_empty() {
        return Err("Select at least one recorded taxon before creating a bundle.".to_string());
    }
    let built = build_tables(request);
    let mut files = match request.format {
        BundleFormat::DarwinCoreArchive => vec![
            BundleFile {
                path: "meta.xml".to_string(),
                media_type: "application/xml".to_string(),
                records: 0,
                columns: Vec::new(),
            },
            BundleFile {
                path: "eml.xml".to_string(),
                media_type: "application/xml".to_string(),
                records: 0,
                columns: Vec::new(),
            },
        ],
        BundleFormat::DarwinCoreDataPackage => vec![
            BundleFile {
                path: "datapackage.json".to_string(),
                media_type: "application/json".to_string(),
                records: 0,
                columns: Vec::new(),
            },
            BundleFile {
                path: "eml.xml".to_string(),
                media_type: "application/xml".to_string(),
                records: 0,
                columns: Vec::new(),
            },
        ],
    };
    let mut media_paths = BTreeSet::new();
    for table in built.tables.all() {
        files.push(BundleFile {
            path: format!("{}.csv", table.name),
            media_type: "text/csv".to_string(),
            records: table.rows.len(),
            columns: table.headers().into_iter().map(str::to_string).collect(),
        });
    }
    for media in &request.media {
        if let Some(path) = &media.source_path
            && Path::new(path).is_file()
            && let Some(output_path) = media_output_path(media)
            && media_paths.insert(output_path.clone())
        {
            files.push(BundleFile {
                path: output_path,
                media_type: "application/octet-stream".to_string(),
                records: 0,
                columns: Vec::new(),
            });
        }
    }
    let mut warnings = request.warnings.clone();
    warnings.extend(built.warnings);
    warnings.extend(media_warnings(&request.media));
    if request.format == BundleFormat::DarwinCoreDataPackage
        && request.archive_format == ArchiveFormat::Zip
    {
        warnings.push(
            "ZIP is a compatibility option. The current DwC-DP guide specifies gzip for \
             whole-package compression."
                .to_string(),
        );
    }
    Ok(BundleManifest {
        format: request.format.clone(),
        archive_format: effective_archive_format(request),
        files,
        warnings,
    })
}

fn write_archive(
    request: &BundleRequest,
    output_path: &Path,
    manifest: &BundleManifest,
) -> Result<(), String> {
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(io_error)?;
    let staging = temporary_directory(parent, "dwca")?;
    let result = (|| {
        let built = build_tables(request);
        write_tables(&staging, &built.tables.all())?;
        fs::write(staging.join("meta.xml"), meta_xml(&built.tables.all())).map_err(io_error)?;
        fs::write(staging.join("eml.xml"), eml_xml(request)).map_err(io_error)?;
        copy_media(&staging, &request.media)?;
        let files = collect_files(&staging)?;
        ZipArchive::new(&staging, None, output_path, &files)
            .write()
            .map_err(io_error)?;
        let _ = manifest;
        Ok(())
    })();
    fs::remove_dir_all(&staging).map_err(io_error)?;
    result
}

fn write_data_package(
    request: &BundleRequest,
    output_path: &Path,
    _manifest: &BundleManifest,
) -> Result<(), String> {
    if output_path.exists() {
        return Err(format!(
            "Data Package destination already exists: {}",
            output_path.display()
        ));
    }
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(io_error)?;
    let staging = temporary_directory(parent, "dwc-dp")?;
    let result = (|| {
        let built = build_tables(request);
        write_tables(&staging, &built.tables.all())?;
        fs::write(staging.join("eml.xml"), eml_xml(request)).map_err(io_error)?;
        let descriptor = data_package_json(request, &built.tables.all());
        let descriptor =
            serde_json::to_vec_pretty(&descriptor).map_err(|error| error.to_string())?;
        fs::write(staging.join("datapackage.json"), descriptor).map_err(io_error)?;
        copy_media(&staging, &request.media)?;
        let files = collect_files(&staging)?;
        match request.archive_format {
            ArchiveFormat::TarGzip => {
                TarGzipArchive::new(&staging, output_path, &files)
                    .write()
                    .map_err(io_error)?;
                let verification = temporary_directory(parent, "dwc-dp-verify")?;
                let verification_result =
                    TarGzipExtractor::new(output_path, &verification).extract();
                let _ = fs::remove_dir_all(&verification);
                verification_result.map_err(io_error)?;
            }
            ArchiveFormat::Zip => {
                ZipArchive::new(&staging, None, output_path, &files)
                    .write()
                    .map_err(io_error)?;
                let verification = temporary_directory(parent, "dwc-dp-verify")?;
                let verification_result = ZipExtractor::new(output_path, &verification).extract();
                let _ = fs::remove_dir_all(&verification);
                verification_result.map_err(io_error)?;
            }
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&staging);
    if result.is_err() {
        let _ = fs::remove_file(output_path);
    }
    result
}

struct Tables {
    occurrences: Table,
    identifications: Option<Table>,
    events: Option<Table>,
    materials: Option<Table>,
    measurements: Option<Table>,
    event_assertions: Option<Table>,
    material_assertions: Option<Table>,
    organism_interactions: Option<Table>,
    organism_interaction_assertions: Option<Table>,
    media: Option<Table>,
    agents: Option<Table>,
    occurrence_agent_roles: Option<Table>,
    event_agent_roles: Option<Table>,
    material_agent_roles: Option<Table>,
    media_agent_roles: Option<Table>,
    occurrence_media: Option<Table>,
}

impl Tables {
    fn all(&self) -> Vec<Table> {
        let mut tables = vec![self.occurrences.clone()];
        for table in [
            &self.identifications,
            &self.events,
            &self.materials,
            &self.measurements,
            &self.event_assertions,
            &self.material_assertions,
            &self.organism_interactions,
            &self.organism_interaction_assertions,
            &self.media,
            &self.agents,
            &self.occurrence_agent_roles,
            &self.event_agent_roles,
            &self.material_agent_roles,
            &self.media_agent_roles,
            &self.occurrence_media,
        ]
        .into_iter()
        .flatten()
        {
            tables.push(table.clone());
        }
        tables
    }
}

/// The bundle tables together with the warnings raised while resolving their columns.
struct BuiltTables {
    tables: Tables,
    warnings: Vec<String>,
}

fn build_tables(request: &BundleRequest) -> BuiltTables {
    let mut builder = TableBuilder::new(&request.format);
    let tables = bundle_tables(request, &mut builder);
    BuiltTables {
        tables,
        warnings: builder.into_warnings(),
    }
}

fn bundle_tables(request: &BundleRequest, builder: &mut TableBuilder) -> Tables {
    let occurrence_rows = normalize_rows(&request.occurrences);
    let material_rows = normalize_rows(&request.materials);
    let measurement_rows = normalize_rows(&request.measurements);
    let occurrence_assertion_rows = normalize_rows(&request.occurrence_assertions);
    let event_assertion_rows = normalize_rows(&request.event_assertions);
    let material_assertion_rows = normalize_rows(&request.material_assertions);
    let interaction_rows = normalize_rows(&request.organism_interactions);
    let interaction_assertion_rows = normalize_rows(&request.organism_interaction_assertions);
    let raw_media_rows = request
        .media
        .iter()
        .map(|media| media.fields.clone())
        .collect::<Vec<_>>();
    let media_rows = normalize_rows(&raw_media_rows);

    if request.format == BundleFormat::DarwinCoreDataPackage {
        let material_occurrences = material_rows
            .iter()
            .filter_map(|row| row.get("occurrenceID").cloned())
            .collect::<BTreeSet<_>>();
        let mut identifications = Vec::new();
        let mut locations: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut relocated_assertions = Vec::new();
        let occurrences = occurrence_rows
            .into_iter()
            .map(|mut row| {
                let occurrence_id = row.get("occurrenceID").cloned().unwrap_or_default();
                if let Some(identification) = identification_row(&mut row, &occurrence_id) {
                    identifications.push(identification);
                }
                relocate_location(&mut row, &mut locations);
                relocated_assertions.extend(occurrence_assertions(
                    &mut row,
                    &occurrence_id,
                    material_occurrences.contains(&occurrence_id),
                ));
                for field in OCCURRENCE_FIELDS_REPRESENTED_ELSEWHERE {
                    row.remove(*field);
                }
                copy_key(&mut row, "occurrenceID", "occurrence_pk");
                copy_key(&mut row, "eventID", "event_fk");
                row.remove("eventID");
                rename_key(&mut row, "individualCount", "organismQuantity");
                if row.contains_key("organismQuantity") {
                    row.insert(
                        "organismQuantityType".to_string(),
                        "individuals".to_string(),
                    );
                }
                row.entry("occurrenceStatus".to_string())
                    .or_insert_with(|| "detected".to_string());
                row
            })
            .collect();
        let mut relocated_event_assertions = Vec::new();
        let events = normalize_rows(&request.events)
            .into_iter()
            .map(|mut row| {
                let event_id = row.get("eventID").cloned().unwrap_or_default();
                for (field, assertion_type) in EVENT_FIELDS_KEPT_AS_ASSERTIONS {
                    if let Some(value) = row.remove(*field) {
                        relocated_event_assertions.push(assertion_row(
                            "eventID",
                            &event_id,
                            assertion_type,
                            value,
                        ));
                    }
                }
                row.remove("eventConductedBy");
                row.remove("eventConductedByID");
                if let Some(location) = locations.get(&event_id) {
                    for (key, value) in location {
                        row.entry(key.clone()).or_insert_with(|| value.clone());
                    }
                }
                copy_key(&mut row, "eventID", "event_pk");
                row.entry("eventCategory".to_string())
                    .or_insert_with(|| "sampling event".to_string());
                row
            })
            .collect();
        let materials = material_rows
            .into_iter()
            .map(|mut row| {
                copy_key(&mut row, "materialEntityID", "materialEntity_pk");
                if let Some(event_id) = row.remove("eventID") {
                    row.insert("collectionEvent_fk".to_string(), event_id);
                }
                if let Some(occurrence_id) = row.remove("occurrenceID") {
                    row.insert("evidenceForOccurrence_fk".to_string(), occurrence_id);
                }
                row
            })
            .collect();
        let mut assertions: Vec<BTreeMap<String, String>> = measurement_rows
            .into_iter()
            .map(|mut row| {
                rename_key(&mut row, "occurrenceID", "occurrence_fk");
                rename_key(&mut row, "measurementID", "assertionID");
                rename_key(&mut row, "measurementType", "assertionType");
                rename_key(&mut row, "measurementValue", "assertionValue");
                rename_key(&mut row, "measurementUnit", "assertionUnit");
                row
            })
            .collect();
        assertions.extend(
            occurrence_assertion_rows
                .into_iter()
                .chain(relocated_assertions)
                .map(|mut row| {
                    rename_key(&mut row, "occurrenceID", "occurrence_fk");
                    row
                }),
        );
        let event_assertions = event_assertion_rows
            .into_iter()
            .chain(relocated_event_assertions)
            .map(|mut row| {
                rename_key(&mut row, "eventID", "event_fk");
                row
            })
            .collect();
        let material_assertions = material_assertion_rows
            .into_iter()
            .map(|mut row| {
                rename_key(&mut row, "materialEntityID", "materialEntity_fk");
                row
            })
            .collect();
        let organism_interactions = interaction_rows
            .into_iter()
            .map(|mut row| {
                copy_key(&mut row, "organismInteractionID", "organismInteraction_pk");
                rename_key(&mut row, "subjectOccurrenceID", "subjectOccurrence_fk");
                rename_key(&mut row, "relatedOccurrenceID", "relatedOccurrence_fk");
                rename_key(&mut row, "eventID", "event_fk");
                row
            })
            .collect();
        let organism_interaction_assertions = interaction_assertion_rows
            .into_iter()
            .map(|mut row| {
                rename_key(&mut row, "organismInteractionID", "organismInteraction_fk");
                row
            })
            .collect();
        let media = dedupe_rows(
            media_rows
                .iter()
                .cloned()
                .map(|mut row| {
                    copy_key(&mut row, "mediaID", "media_pk");
                    row.remove("occurrenceID");
                    row.remove("creatorID");
                    row.remove("creator");
                    row.remove("created");
                    row.remove("description");
                    row
                })
                .collect(),
            "media_pk",
        );
        let occurrence_media = media_rows
            .iter()
            .filter_map(|row| {
                Some(BTreeMap::from([
                    ("media_fk".to_string(), row.get("mediaID")?.clone()),
                    (
                        "occurrence_fk".to_string(),
                        row.get("occurrenceID")?.clone(),
                    ),
                ]))
            })
            .collect();
        let agents = normalize_rows(&request.agents)
            .into_iter()
            .map(|mut row| {
                copy_key(&mut row, "agentID", "agent_pk");
                row
            })
            .collect();

        return Tables {
            occurrences: builder.table(
                "occurrence",
                "https://rs.tdwg.org/dwc-dp/terms/Occurrence",
                true,
                occurrences,
                &[
                    "occurrence_pk",
                    "occurrenceID",
                    "event_fk",
                    "occurrenceStatus",
                ],
            ),
            identifications: builder.optional_table(
                "identification",
                "https://rs.tdwg.org/dwc-dp/terms/Identification",
                identifications,
                &["identification_pk", "identificationID", "occurrence_fk"],
            ),
            events: builder.optional_table(
                "event",
                "https://rs.tdwg.org/dwc-dp/terms/Event",
                events,
                &["event_pk", "eventID", "eventCategory"],
            ),
            materials: builder.optional_table(
                "material",
                "http://rs.tdwg.org/dwc/terms/MaterialEntity",
                materials,
                &["materialEntity_pk", "materialEntityID"],
            ),
            measurements: builder.optional_table(
                "occurrence-assertion",
                "http://rs.tdwg.org/dwc/terms/Assertion",
                assertions,
                &[
                    "occurrence_fk",
                    "assertionID",
                    "assertionType",
                    "assertionValue",
                ],
            ),
            event_assertions: builder.optional_table(
                "event-assertion",
                "http://rs.tdwg.org/dwc/terms/Assertion",
                event_assertions,
                &["event_fk", "assertionID", "assertionType", "assertionValue"],
            ),
            material_assertions: builder.optional_table(
                "material-assertion",
                "http://rs.tdwg.org/dwc/terms/Assertion",
                material_assertions,
                &[
                    "materialEntity_fk",
                    "assertionID",
                    "assertionType",
                    "assertionValue",
                ],
            ),
            organism_interactions: builder.optional_table(
                "organism-interaction",
                "https://rs.tdwg.org/dwc-dp/terms/OrganismInteraction",
                organism_interactions,
                &[
                    "organismInteraction_pk",
                    "organismInteractionID",
                    "subjectOccurrence_fk",
                    "relatedOccurrence_fk",
                    "organismInteractionType",
                ],
            ),
            organism_interaction_assertions: builder.optional_table(
                "organism-interaction-assertion",
                "http://rs.tdwg.org/dwc/terms/Assertion",
                organism_interaction_assertions,
                &[
                    "organismInteraction_fk",
                    "assertionID",
                    "assertionType",
                    "assertionValue",
                ],
            ),
            media: builder.optional_table(
                "media",
                "http://rs.tdwg.org/ac/terms/Media",
                media,
                &["media_pk", "mediaID"],
            ),
            agents: builder.optional_table(
                "agent",
                "http://purl.org/dc/terms/Agent",
                agents,
                &["agent_pk", "agentID", "agentType", "preferredAgentName"],
            ),
            occurrence_agent_roles: builder.role_table(
                "occurrence-agent-role",
                normalize_rows(&request.occurrence_agent_roles),
                "occurrenceID",
                "occurrence_fk",
            ),
            event_agent_roles: builder.role_table(
                "event-agent-role",
                normalize_rows(&request.event_agent_roles),
                "eventID",
                "event_fk",
            ),
            material_agent_roles: builder.role_table(
                "material-agent-role",
                normalize_rows(&request.material_agent_roles),
                "materialEntityID",
                "materialEntity_fk",
            ),
            media_agent_roles: builder.role_table(
                "media-agent-role",
                normalize_rows(&request.media_agent_roles),
                "mediaID",
                "media_fk",
            ),
            occurrence_media: builder.optional_table(
                "occurrence-media",
                "http://rs.tdwg.org/ac/terms/Media",
                occurrence_media,
                &["media_fk", "occurrence_fk"],
            ),
        };
    }

    let media_rows = media_rows
        .into_iter()
        .map(darwin_core_archive_media_row)
        .collect();
    let occurrences = builder.table(
        "occurrence",
        "http://rs.tdwg.org/dwc/terms/Occurrence",
        true,
        occurrence_rows.clone(),
        &["occurrenceID", "basisOfRecord"],
    );
    let materials = builder.optional_table(
        "material",
        "http://rs.tdwg.org/dwc/terms/MaterialEntity",
        material_rows.clone(),
        &["occurrenceID"],
    );
    let mut archive_measurements = measurement_rows;
    archive_measurements.extend(archive_assertions(
        occurrence_assertion_rows,
        &occurrence_rows,
        &material_rows,
        &interaction_rows,
    ));
    archive_measurements.extend(archive_assertions(
        event_assertion_rows,
        &occurrence_rows,
        &material_rows,
        &interaction_rows,
    ));
    archive_measurements.extend(archive_assertions(
        material_assertion_rows,
        &occurrence_rows,
        &material_rows,
        &interaction_rows,
    ));
    archive_measurements.extend(archive_assertions(
        interaction_assertion_rows,
        &occurrence_rows,
        &material_rows,
        &interaction_rows,
    ));
    let measurements = builder.optional_table(
        "measurement_or_fact",
        "http://rs.tdwg.org/dwc/terms/MeasurementOrFact",
        archive_measurements,
        &["occurrenceID"],
    );
    let media = builder.optional_table(
        "multimedia",
        "http://rs.gbif.org/terms/1.0/Multimedia",
        media_rows,
        &["occurrenceID"],
    );
    let resource_relationships = builder.optional_table(
        "resource_relationship",
        "http://rs.gbif.org/terms/1.0/ResourceRelationship",
        interaction_rows
            .into_iter()
            .map(|mut row| {
                rename_key(&mut row, "organismInteractionID", "resourceRelationshipID");
                rename_key(&mut row, "subjectOccurrenceID", "resourceID");
                rename_key(&mut row, "relatedOccurrenceID", "relatedResourceID");
                rename_key(
                    &mut row,
                    "organismInteractionType",
                    "relationshipOfResource",
                );
                rename_key(&mut row, "relatedOrganismPart", "relationshipRemarks");
                row.remove("eventID");
                row.remove("organismInteractionDescription");
                row
            })
            .collect(),
        &["resourceID", "relatedResourceID", "relationshipOfResource"],
    );
    Tables {
        occurrences,
        identifications: None,
        events: None,
        materials,
        measurements,
        event_assertions: None,
        material_assertions: None,
        organism_interactions: resource_relationships,
        organism_interaction_assertions: None,
        media,
        agents: None,
        occurrence_agent_roles: None,
        event_agent_roles: None,
        material_agent_roles: None,
        media_agent_roles: None,
        occurrence_media: None,
    }
}

/// Moves the determination values a Data Package carries on its `identification` table.
///
/// Rank values leave the occurrence row; the accepted determination is repeated, because
/// the Data Package `occurrence` class carries it as well.
fn identification_row(
    row: &mut BTreeMap<String, String>,
    occurrence_id: &str,
) -> Option<BTreeMap<String, String>> {
    let mut identification = BTreeMap::new();
    for field in IDENTIFICATION_ONLY_FIELDS {
        if let Some(value) = row.remove(*field) {
            identification.insert((*field).to_string(), value);
        }
    }
    for field in IDENTIFICATION_SHARED_FIELDS {
        if let Some(value) = row.get(*field) {
            identification.insert((*field).to_string(), value.clone());
        }
    }
    if identification.is_empty() || occurrence_id.is_empty() {
        return None;
    }
    let identification_id = format!("{occurrence_id}:identification");
    identification.insert("identificationID".to_string(), identification_id.clone());
    identification.insert("identification_pk".to_string(), identification_id);
    identification.insert("occurrence_fk".to_string(), occurrence_id.to_string());
    Some(identification)
}

/// Moves the location values a Data Package carries on its `event` table.
fn relocate_location(
    row: &mut BTreeMap<String, String>,
    locations: &mut BTreeMap<String, BTreeMap<String, String>>,
) {
    let event_id = row.get("eventID").cloned();
    for field in EVENT_LOCATION_FIELDS {
        let Some(value) = row.remove(*field) else {
            continue;
        };
        let Some(event_id) = event_id.clone() else {
            continue;
        };
        locations
            .entry(event_id)
            .or_default()
            .entry((*field).to_string())
            .or_insert(value);
    }
}

/// Keeps occurrence values that the Data Package has no class column for as assertions.
///
/// Catalog and preparation values are dropped instead when a material row already carries
/// them for the same occurrence.
fn occurrence_assertions(
    row: &mut BTreeMap<String, String>,
    occurrence_id: &str,
    has_material: bool,
) -> Vec<BTreeMap<String, String>> {
    let mut assertions = Vec::new();
    if let Some(value) = row.remove("associatedTaxa") {
        assertions.push(assertion_row(
            "occurrenceID",
            occurrence_id,
            "associated taxa",
            value,
        ));
    }
    for (field, assertion_type) in OCCURRENCE_FIELDS_KEPT_AS_ASSERTIONS {
        let Some(value) = row.remove(*field) else {
            continue;
        };
        if has_material {
            continue;
        }
        assertions.push(assertion_row(
            "occurrenceID",
            occurrence_id,
            assertion_type,
            value,
        ));
    }
    assertions
}

fn assertion_row(
    owner_key: &str,
    owner_id: &str,
    assertion_type: &str,
    value: String,
) -> BTreeMap<String, String> {
    let slug = assertion_type.replace(' ', "-");
    BTreeMap::from([
        (owner_key.to_string(), owner_id.to_string()),
        ("assertionID".to_string(), format!("{owner_id}:{slug}")),
        ("assertionType".to_string(), assertion_type.to_string()),
        ("assertionValue".to_string(), value),
    ])
}

fn archive_assertions(
    rows: Vec<BTreeMap<String, String>>,
    occurrences: &[BTreeMap<String, String>],
    materials: &[BTreeMap<String, String>],
    interactions: &[BTreeMap<String, String>],
) -> Vec<BTreeMap<String, String>> {
    rows.into_iter()
        .filter_map(|mut row| {
            let occurrence_id = row
                .remove("occurrenceID")
                .or_else(|| {
                    let event_id = row.get("eventID")?;
                    occurrences
                        .iter()
                        .find(|occurrence| occurrence.get("eventID") == Some(event_id))
                        .and_then(|occurrence| occurrence.get("occurrenceID").cloned())
                })
                .or_else(|| {
                    let material_id = row.get("materialEntityID")?;
                    materials
                        .iter()
                        .find(|material| material.get("materialEntityID") == Some(material_id))
                        .and_then(|material| material.get("occurrenceID").cloned())
                })
                .or_else(|| {
                    let interaction_id = row.get("organismInteractionID")?;
                    interactions
                        .iter()
                        .find(|interaction| {
                            interaction.get("organismInteractionID") == Some(interaction_id)
                        })
                        .and_then(|interaction| interaction.get("subjectOccurrenceID").cloned())
                })?;
            row.insert("occurrenceID".to_string(), occurrence_id);
            rename_key(&mut row, "assertionID", "measurementID");
            rename_key(&mut row, "assertionType", "measurementType");
            rename_key(&mut row, "assertionValue", "measurementValue");
            rename_key(&mut row, "assertionUnit", "measurementUnit");
            row.remove("eventID");
            row.remove("materialEntityID");
            row.remove("organismInteractionID");
            Some(row)
        })
        .collect()
}

fn copy_key(row: &mut BTreeMap<String, String>, source: &str, target: &str) {
    if let Some(value) = row.get(source).cloned() {
        row.insert(target.to_string(), value);
    }
}

fn rename_key(row: &mut BTreeMap<String, String>, source: &str, target: &str) {
    if let Some(value) = row.remove(source) {
        row.insert(target.to_string(), value);
    }
}

fn dedupe_rows(rows: Vec<BTreeMap<String, String>>, key: &str) -> Vec<BTreeMap<String, String>> {
    let mut unique = BTreeMap::new();
    for row in rows {
        if let Some(value) = row.get(key) {
            unique.entry(value.clone()).or_insert(row);
        }
    }
    unique.into_values().collect()
}

fn darwin_core_archive_media_row(mut row: BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mappings = [
        ("mediaID", "dcterms:identifier"),
        ("mediaType", "dcterms:type"),
        ("title", "dcterms:title"),
        ("created", "dcterms:created"),
        ("creator", "dcterms:creator"),
        ("description", "dcterms:description"),
        ("accessURI", "ac:accessURI"),
    ];
    for (source, target) in mappings {
        rename_key(&mut row, source, target);
    }
    row.remove("creatorID");
    row
}

fn has_value(row: &BTreeMap<String, Value>, key: &str) -> bool {
    row.get(key).and_then(value_to_string).is_some()
}

fn validate_relationships(tables: &Tables) -> Vec<String> {
    let mut errors = Vec::new();
    let occurrence_ids = key_values(&tables.occurrences, "occurrence_pk");
    let event_ids = tables
        .events
        .as_ref()
        .map(|table| key_values(table, "event_pk"))
        .unwrap_or_default();
    let material_ids = tables
        .materials
        .as_ref()
        .map(|table| key_values(table, "materialEntity_pk"))
        .unwrap_or_default();
    let interaction_ids = tables
        .organism_interactions
        .as_ref()
        .map(|table| key_values(table, "organismInteraction_pk"))
        .unwrap_or_default();
    for row in &tables.occurrences.rows {
        if let Some(event_fk) = row.get("event_fk")
            && !event_ids.contains(event_fk)
        {
            errors.push(format!(
                "Occurrence {} references missing event {event_fk}.",
                row.get("occurrenceID").map_or("unknown", String::as_str)
            ));
        }
    }
    for table in [
        &tables.measurements,
        &tables.occurrence_agent_roles,
        &tables.occurrence_media,
    ]
    .into_iter()
    .flatten()
    {
        for row in &table.rows {
            if let Some(value) = row.get("occurrence_fk")
                && !occurrence_ids.contains(value)
            {
                errors.push(format!(
                    "{} references missing occurrence {value}.",
                    table.name
                ));
            }
        }
    }
    for table in [&tables.event_assertions].into_iter().flatten() {
        for row in &table.rows {
            if let Some(value) = row.get("event_fk")
                && !event_ids.contains(value)
            {
                errors.push(format!("{} references missing event {value}.", table.name));
            }
        }
    }
    for table in [&tables.material_assertions].into_iter().flatten() {
        for row in &table.rows {
            if let Some(value) = row.get("materialEntity_fk")
                && !material_ids.contains(value)
            {
                errors.push(format!(
                    "{} references missing material {value}.",
                    table.name
                ));
            }
        }
    }
    if let Some(table) = &tables.organism_interactions {
        for row in &table.rows {
            for key in ["subjectOccurrence_fk", "relatedOccurrence_fk"] {
                if let Some(value) = row.get(key)
                    && !occurrence_ids.contains(value)
                {
                    errors.push(format!(
                        "{} references missing occurrence {value}.",
                        table.name
                    ));
                }
            }
        }
    }
    for table in [&tables.organism_interaction_assertions]
        .into_iter()
        .flatten()
    {
        for row in &table.rows {
            if let Some(value) = row.get("organismInteraction_fk")
                && !interaction_ids.contains(value)
            {
                errors.push(format!(
                    "{} references missing organism interaction {value}.",
                    table.name
                ));
            }
        }
    }
    errors
}

fn key_values(table: &Table, key: &str) -> BTreeSet<String> {
    table
        .rows
        .iter()
        .filter_map(|row| row.get(key).cloned())
        .collect()
}

/// Builds bundle tables, keeping only the columns the term registry recognizes.
///
/// A candidate header with no registered standard term for its table and profile is
/// withheld rather than published under an invented term URI, and is reported to the user
/// through the bundle manifest.
struct TableBuilder {
    profile: BundleProfile,
    withheld: BTreeMap<&'static str, BTreeSet<String>>,
}

impl TableBuilder {
    pub fn new(format: &BundleFormat) -> Self {
        Self {
            profile: BundleProfile::from(format),
            withheld: BTreeMap::new(),
        }
    }

    pub fn table(
        &mut self,
        name: &'static str,
        row_type: &'static str,
        core: bool,
        rows: Vec<BTreeMap<String, String>>,
        required: &[&str],
    ) -> Table {
        let columns = self.columns(name, &rows, required);
        Table {
            name,
            row_type,
            core,
            columns,
            rows,
        }
    }

    pub fn optional_table(
        &mut self,
        name: &'static str,
        row_type: &'static str,
        rows: Vec<BTreeMap<String, String>>,
        required: &[&str],
    ) -> Option<Table> {
        (!rows.is_empty()).then(|| self.table(name, row_type, false, rows, required))
    }

    pub fn role_table(
        &mut self,
        name: &'static str,
        rows: Vec<BTreeMap<String, String>>,
        target_source: &str,
        target_key: &str,
    ) -> Option<Table> {
        let rows = rows
            .into_iter()
            .map(|mut row| {
                rename_key(&mut row, target_source, target_key);
                rename_key(&mut row, "agentID", "agent_fk");
                row
            })
            .collect();
        self.optional_table(
            name,
            "http://rs.tdwg.org/dwc/terms/AgentRole",
            rows,
            &[target_key, "agent_fk", "agentRole", "agentRoleOrder"],
        )
    }

    /// One warning per table naming the values that were withheld from the bundle.
    pub fn into_warnings(self) -> Vec<String> {
        self.withheld
            .into_iter()
            .map(|(table, headers)| {
                let names = headers.into_iter().collect::<Vec<_>>();
                format!(
                    "{table}: {} field(s) were not written because they have no registered \
                     Darwin Core term ({}). Export a NAHPU Data Package to keep them.",
                    names.len(),
                    names.join(", ")
                )
            })
            .collect()
    }

    fn columns(
        &mut self,
        name: &'static str,
        rows: &[BTreeMap<String, String>],
        required: &[&str],
    ) -> Vec<Column> {
        let mut candidates = required
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();
        let mut optional = BTreeSet::new();
        for row in rows {
            for (key, value) in row {
                if !value.is_empty() && !required.contains(&key.as_str()) {
                    optional.insert(key.clone());
                }
            }
        }
        candidates.extend(optional);
        let mut columns = Vec::new();
        for header in candidates {
            match TermRegistry::column(name, &header, self.profile) {
                Some(term) => columns.push(Column::new(term)),
                None => {
                    debug_assert!(
                        !required.contains(&header.as_str()),
                        "required column {name}.{header} is not registered"
                    );
                    self.withheld.entry(name).or_default().insert(header);
                }
            }
        }
        columns
    }
}

impl From<&BundleFormat> for BundleProfile {
    fn from(value: &BundleFormat) -> Self {
        match value {
            BundleFormat::DarwinCoreArchive => Self::Archive,
            BundleFormat::DarwinCoreDataPackage => Self::DataPackage,
        }
    }
}

fn normalize_rows(rows: &[BTreeMap<String, Value>]) -> Vec<BTreeMap<String, String>> {
    rows.iter().map(normalize_row).collect()
}

fn normalize_row(row: &BTreeMap<String, Value>) -> BTreeMap<String, String> {
    row.iter()
        .filter_map(|(key, value)| value_to_string(value).map(|value| (key.clone(), value)))
        .collect()
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) if value.trim().is_empty() => None,
        Value::String(value) => Some(value.trim().to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => Some(value.to_string()),
    }
}

fn write_tables(output_dir: &Path, tables: &[Table]) -> Result<(), String> {
    for table in tables {
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_path(output_dir.join(format!("{}.csv", table.name)))
            .map_err(|error| error.to_string())?;
        writer
            .write_record(table.headers())
            .map_err(|error| error.to_string())?;
        for row in &table.rows {
            let record = table
                .columns
                .iter()
                .map(|column| row.get(&column.header).map_or("", String::as_str));
            writer
                .write_record(record)
                .map_err(|error| error.to_string())?;
        }
        writer.flush().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn meta_xml(tables: &[Table]) -> String {
    let core = tables
        .iter()
        .find(|table| table.core)
        .expect("occurrence core is always present");
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<archive xmlns=\"http://rs.tdwg.org/dwc/text/\" metadata=\"eml.xml\">\n");
    xml.push_str(&meta_table(core, "core"));
    for table in tables.iter().filter(|table| !table.core) {
        xml.push_str(&meta_table(table, "extension"));
    }
    xml.push_str("</archive>\n");
    xml
}

fn meta_table(table: &Table, element: &str) -> String {
    let mut xml = format!(
        "  <{element} encoding=\"UTF-8\" fieldsTerminatedBy=\",\" linesTerminatedBy=\"\\n\" ignoreHeaderLines=\"1\" rowType=\"{}\">\n    <files><location>{}.csv</location></files>\n",
        table.row_type, table.name
    );
    if table.core {
        xml.push_str("    <id index=\"0\"/>\n");
    } else {
        xml.push_str("    <coreid index=\"0\"/>\n");
    }
    let first_field = usize::from(!table.core);
    for (index, column) in table.columns.iter().enumerate().skip(first_field) {
        xml.push_str(&format!(
            "    <field index=\"{index}\" term=\"{}\"/>\n",
            column.term_uri
        ));
    }
    xml.push_str(&format!("  </{element}>\n"));
    xml
}

fn data_package_json(request: &BundleRequest, tables: &[Table]) -> Value {
    let resources = tables
        .iter()
        .map(|table| {
            let fields = table
                .columns
                .iter()
                .map(|column| field_descriptor(table.name, column))
                .collect::<Vec<_>>();
            let mut schema =
                serde_json::Map::from_iter([("fields".to_string(), Value::Array(fields))]);
            if let Some(primary_key) = primary_key(table.name) {
                schema.insert(
                    "primaryKey".to_string(),
                    Value::String(primary_key.to_string()),
                );
            }
            let foreign_keys = foreign_keys(table.name);
            if !foreign_keys.is_empty() {
                schema.insert("foreignKeys".to_string(), Value::Array(foreign_keys));
            }
            Value::Object(serde_json::Map::from_iter([
                ("name".to_string(), Value::String(table.name.to_string())),
                (
                    "path".to_string(),
                    Value::String(format!("{}.csv", table.name)),
                ),
                (
                    "profile".to_string(),
                    Value::String("tabular-data-resource".to_string()),
                ),
                ("format".to_string(), Value::String("csv".to_string())),
                (
                    "mediatype".to_string(),
                    Value::String("text/csv".to_string()),
                ),
                ("schema".to_string(), Value::Object(schema)),
            ]))
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "name": package_name(&request.name),
        "profile": DWC_DP_PROFILE,
        "resources": resources,
    })
}

fn primary_key(table_name: &str) -> Option<&'static str> {
    match table_name {
        "event" => Some("event_pk"),
        "identification" => Some("identification_pk"),
        "occurrence" => Some("occurrence_pk"),
        "material" => Some("materialEntity_pk"),
        "agent" => Some("agent_pk"),
        "media" => Some("media_pk"),
        "organism-interaction" => Some("organismInteraction_pk"),
        _ => None,
    }
}

fn foreign_keys(table_name: &str) -> Vec<Value> {
    let relationships: &[(&str, &str, &str, &str)] = match table_name {
        "occurrence" => &[("event_fk", "happened during", "event", "event_pk")],
        "identification" => &[("occurrence_fk", "about", "occurrence", "occurrence_pk")],
        "material" => &[
            (
                "collectionEvent_fk",
                "collected during",
                "event",
                "event_pk",
            ),
            (
                "evidenceForOccurrence_fk",
                "evidence for",
                "occurrence",
                "occurrence_pk",
            ),
        ],
        "occurrence-assertion" => &[("occurrence_fk", "about", "occurrence", "occurrence_pk")],
        "event-assertion" => &[("event_fk", "about", "event", "event_pk")],
        "material-assertion" => &[(
            "materialEntity_fk",
            "about",
            "material",
            "materialEntity_pk",
        )],
        "organism-interaction" => &[
            (
                "subjectOccurrence_fk",
                "subject occurrence",
                "occurrence",
                "occurrence_pk",
            ),
            (
                "relatedOccurrence_fk",
                "related occurrence",
                "occurrence",
                "occurrence_pk",
            ),
            ("event_fk", "happened during", "event", "event_pk"),
        ],
        "organism-interaction-assertion" => &[(
            "organismInteraction_fk",
            "about",
            "organism-interaction",
            "organismInteraction_pk",
        )],
        "occurrence-agent-role" => &[
            ("occurrence_fk", "role for", "occurrence", "occurrence_pk"),
            ("agent_fk", "role holder", "agent", "agent_pk"),
        ],
        "event-agent-role" => &[
            ("event_fk", "role for", "event", "event_pk"),
            ("agent_fk", "role holder", "agent", "agent_pk"),
        ],
        "material-agent-role" => &[
            (
                "materialEntity_fk",
                "role for",
                "material",
                "materialEntity_pk",
            ),
            ("agent_fk", "role holder", "agent", "agent_pk"),
        ],
        "media-agent-role" => &[
            ("media_fk", "role for", "media", "media_pk"),
            ("agent_fk", "role holder", "agent", "agent_pk"),
        ],
        "occurrence-media" => &[
            ("media_fk", "this media instance", "media", "media_pk"),
            ("occurrence_fk", "about", "occurrence", "occurrence_pk"),
        ],
        _ => &[],
    };
    relationships
        .iter()
        .map(|(field, predicate, resource, reference_field)| {
            serde_json::json!({
                "fields": field,
                "predicate": predicate,
                "reference": {
                    "resource": resource,
                    "fields": reference_field,
                },
            })
        })
        .collect()
}

fn field_descriptor(table_name: &str, column: &Column) -> Value {
    let field_type = if column.integer { "integer" } else { "string" };
    serde_json::json!({
        "name": column.header,
        "title": field_title(&column.header),
        "description": format!(
            "{} field in the {table_name} table.",
            field_title(&column.header)
        ),
        "type": field_type,
        "format": "default",
        "dcterms:isVersionOf": column.term_uri,
    })
}

fn field_title(value: &str) -> String {
    let value = value
        .strip_suffix("_pk")
        .or_else(|| value.strip_suffix("_fk"))
        .unwrap_or(value);
    let mut title = String::new();
    let mut previous_was_lowercase = false;
    for character in value.chars() {
        if character.is_ascii_uppercase() && previous_was_lowercase {
            title.push(' ');
        }
        title.push(character);
        previous_was_lowercase = character.is_ascii_lowercase();
    }
    let mut characters = title.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => title,
    }
}

fn package_name(name: &str) -> String {
    let normalized = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    normalized.trim_matches('-').to_string()
}

fn eml_xml(request: &BundleRequest) -> String {
    let title = xml_escape(
        request
            .project
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(&request.name),
    );
    let description = xml_escape(
        request
            .project
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<eml:eml xmlns:eml=\"https://eml.ecoinformatics.org/eml-2.2.0\">\n  <dataset>\n    <title>{title}</title>\n    <abstract><para>{description}</para></abstract>\n  </dataset>\n</eml:eml>\n"
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn copy_media(output_dir: &Path, media: &[BtreeMedia]) -> Result<(), String> {
    let mut copied = BTreeSet::new();
    for entry in media {
        let Some(source_path) = &entry.source_path else {
            continue;
        };
        let source = Path::new(source_path);
        if !source.is_file() {
            continue;
        }
        let Some(relative_path) = media_output_path(entry) else {
            continue;
        };
        if !copied.insert(relative_path.clone()) {
            continue;
        }
        let destination = output_dir.join(relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        fs::copy(source, destination).map_err(io_error)?;
    }
    Ok(())
}

fn media_output_path(media: &BtreeMedia) -> Option<String> {
    if let Some(access_uri) = media.fields.get("accessURI").and_then(Value::as_str)
        && Path::new(access_uri).is_relative()
        && !access_uri.split('/').any(|part| part == "..")
    {
        return Some(access_uri.replace('\\', "/"));
    }
    let source = Path::new(media.source_path.as_ref()?);
    let file_name = source.file_name()?.to_str()?;
    Some(format!("media/{file_name}"))
}

fn collect_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files(&path)?);
        } else {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn temporary_directory(parent: &Path, prefix: &str) -> Result<PathBuf, String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let directory = parent.join(format!(".{prefix}-{nonce}"));
    fs::create_dir(&directory).map_err(io_error)?;
    Ok(directory)
}

fn media_warnings(media: &[BtreeMedia]) -> Vec<String> {
    media
        .iter()
        .filter_map(|entry| {
            entry
                .source_path
                .as_ref()
                .filter(|path| !Path::new(path).is_file())
                .map(|path| format!("Media file was not found and was not bundled: {path}"))
        })
        .collect()
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn request(format: BundleFormat) -> BundleRequest {
        BundleRequest {
            format,
            archive_format: ArchiveFormat::TarGzip,
            name: "NAHPU specimen data".to_string(),
            project: BTreeMap::new(),
            occurrences: vec![BTreeMap::from([
                (
                    "occurrenceID".to_string(),
                    Value::String("occ-1".to_string()),
                ),
                (
                    "basisOfRecord".to_string(),
                    Value::String("PreservedSpecimen".to_string()),
                ),
                (
                    "scientificName".to_string(),
                    Value::String("Testus example".to_string()),
                ),
                (
                    "scientificNameAuthorship".to_string(),
                    Value::String(String::new()),
                ),
            ])],
            events: Vec::new(),
            materials: Vec::new(),
            measurements: Vec::new(),
            occurrence_assertions: Vec::new(),
            event_assertions: Vec::new(),
            material_assertions: Vec::new(),
            organism_interactions: Vec::new(),
            organism_interaction_assertions: Vec::new(),
            media: Vec::new(),
            agents: Vec::new(),
            occurrence_agent_roles: Vec::new(),
            event_agent_roles: Vec::new(),
            material_agent_roles: Vec::new(),
            media_agent_roles: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn unregistered_columns_are_withheld_and_reported() {
        for format in [
            BundleFormat::DarwinCoreArchive,
            BundleFormat::DarwinCoreDataPackage,
        ] {
            let mut request = request(format);
            request.occurrences[0].insert(
                "dynamicProperties".to_string(),
                Value::String("{\"x\":1}".to_string()),
            );
            request.occurrences[0]
                .insert("notATerm".to_string(), Value::String("value".to_string()));
            let manifest = build_manifest(&request).unwrap();
            let occurrence = manifest
                .files
                .iter()
                .find(|file| file.path == "occurrence.csv")
                .unwrap();
            assert!(
                !occurrence
                    .columns
                    .contains(&"dynamicProperties".to_string())
            );
            assert!(!occurrence.columns.contains(&"notATerm".to_string()));
            let warning = manifest
                .warnings
                .iter()
                .find(|warning| warning.starts_with("occurrence:"))
                .expect("withheld columns are reported");
            assert!(warning.contains("dynamicProperties"));
            assert!(warning.contains("notATerm"));
            assert!(warning.contains("NAHPU Data Package"));
        }
    }

    #[test]
    fn archive_meta_advertises_only_registered_term_uris() {
        let tables = build_tables(&request(BundleFormat::DarwinCoreArchive)).tables;
        let meta = meta_xml(&tables.all());
        let registered = tables
            .all()
            .into_iter()
            .flat_map(|table| {
                table
                    .columns
                    .into_iter()
                    .map(|column| column.term_uri)
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeSet<_>>();
        for fragment in meta.split("term=\"").skip(1) {
            let uri = fragment.split('"').next().unwrap();
            assert!(registered.contains(uri), "{uri} is not a registered term");
        }
    }

    #[test]
    fn data_package_relocates_determination_and_location_values() {
        let mut request = request(BundleFormat::DarwinCoreDataPackage);
        let occurrence = &mut request.occurrences[0];
        occurrence.insert("eventID".to_string(), Value::String("ev-1".to_string()));
        occurrence.insert("genus".to_string(), Value::String("Testus".to_string()));
        occurrence.insert(
            "decimalLatitude".to_string(),
            Value::String("1.5".to_string()),
        );
        occurrence.insert(
            "associatedTaxa".to_string(),
            Value::String("host: Rattus".to_string()),
        );
        request.events = vec![BTreeMap::from([
            ("eventID".to_string(), Value::String("ev-1".to_string())),
            (
                "samplingProtocol".to_string(),
                Value::String("mist net".to_string()),
            ),
        ])];
        let tables = build_tables(&request).tables;

        let occurrence = &tables.occurrences;
        assert!(!occurrence.headers().contains(&"genus"));
        assert!(!occurrence.headers().contains(&"decimalLatitude"));
        assert!(!occurrence.headers().contains(&"basisOfRecord"));
        assert!(occurrence.headers().contains(&"occurrence_pk"));

        let identification = tables.identifications.expect("identification table");
        assert!(identification.headers().contains(&"genus"));
        assert_eq!(
            identification.rows[0].get("occurrence_fk").unwrap(),
            "occ-1"
        );

        let event = tables.events.expect("event table");
        assert!(event.headers().contains(&"decimalLatitude"));
        assert!(!event.headers().contains(&"samplingProtocol"));

        let event_assertions = tables.event_assertions.expect("event assertions");
        assert!(event_assertions.rows.iter().any(|row| {
            row.get("assertionType").map(String::as_str) == Some("sampling protocol")
        }));
        let occurrence_assertions = tables.measurements.expect("occurrence assertions");
        assert!(occurrence_assertions.rows.iter().any(|row| {
            row.get("assertionType").map(String::as_str) == Some("associated taxa")
        }));
    }

    #[test]
    fn data_package_only_columns_never_reach_the_archive() {
        let tables = build_tables(&request(BundleFormat::DarwinCoreArchive)).tables;
        for table in tables.all() {
            for header in table.headers() {
                assert!(
                    !header.ends_with("_pk") && !header.ends_with("_fk"),
                    "{} carries the Data Package key {header}",
                    table.name
                );
            }
        }
    }

    #[test]
    fn plan_omits_empty_columns() {
        let manifest = build_manifest(&request(BundleFormat::DarwinCoreArchive)).unwrap();
        let occurrence = manifest
            .files
            .iter()
            .find(|file| file.path == "occurrence.csv")
            .unwrap();
        assert!(
            !occurrence
                .columns
                .contains(&"scientificNameAuthorship".to_string())
        );
    }

    #[test]
    fn writes_darwin_core_archive() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("records.zip");
        write_bundle(&request(BundleFormat::DarwinCoreArchive), &output).unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(output).unwrap()).unwrap();
        assert!(zip.by_name("meta.xml").is_ok());
        assert!(zip.by_name("occurrence.csv").is_ok());
    }

    #[test]
    fn writes_tar_gzip_data_package() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("records.dwc-dp.tar.gz");
        write_bundle(&request(BundleFormat::DarwinCoreDataPackage), &output).unwrap();
        let extracted = directory.path().join("extracted");
        TarGzipExtractor::new(&output, &extracted)
            .extract()
            .unwrap();
        let descriptor: Value =
            serde_json::from_slice(&fs::read(extracted.join("datapackage.json")).unwrap()).unwrap();
        assert_eq!(descriptor["profile"], DWC_DP_PROFILE);
    }

    #[test]
    fn writes_zip_data_package() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("records.dwc-dp.zip");
        let mut request = request(BundleFormat::DarwinCoreDataPackage);
        request.archive_format = ArchiveFormat::Zip;
        write_bundle(&request, &output).unwrap();
        let extracted = directory.path().join("extracted");
        ZipExtractor::new(&output, &extracted).extract().unwrap();
        assert!(extracted.join("datapackage.json").is_file());
    }

    #[test]
    fn validates_data_package_occurrence_id_before_internal_key_derivation() {
        let request = request(BundleFormat::DarwinCoreDataPackage);
        let input = serde_json::to_string(&request).unwrap();
        assert_eq!(validate_bundle_json(&input).unwrap(), "[]");

        let tables = build_tables(&request).tables;
        let row = &tables.occurrences.rows[0];
        assert_eq!(row["occurrenceID"], "occ-1");
        assert_eq!(row["occurrence_pk"], "occ-1");
    }

    #[test]
    fn data_package_uses_official_profile_and_complete_field_descriptors() {
        let request = request(BundleFormat::DarwinCoreDataPackage);
        let tables = build_tables(&request).tables;
        let descriptor = data_package_json(&request, &tables.all());

        assert_eq!(descriptor["profile"], DWC_DP_PROFILE);
        let occurrence = descriptor["resources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|resource| resource["name"] == "occurrence")
            .unwrap();
        let fields = occurrence["schema"]["fields"].as_array().unwrap();
        let occurrence_id = fields
            .iter()
            .find(|field| field["name"] == "occurrenceID")
            .unwrap();
        assert_eq!(occurrence_id["title"], "Occurrence ID");
        assert!(occurrence_id["description"].as_str().unwrap().len() > 5);
        assert_eq!(
            occurrence_id["dcterms:isVersionOf"],
            "http://rs.tdwg.org/dwc/terms/occurrenceID"
        );
    }

    #[test]
    fn manifest_does_not_list_missing_media_files() {
        let mut request = request(BundleFormat::DarwinCoreArchive);
        request.media.push(BtreeMedia {
            fields: BTreeMap::from([(
                "accessURI".to_string(),
                Value::String("media/missing.jpg".to_string()),
            )]),
            source_path: Some("/definitely/missing.jpg".to_string()),
        });

        let manifest = build_manifest(&request).unwrap();
        assert!(
            manifest
                .files
                .iter()
                .all(|file| file.path != "media/missing.jpg")
        );
        assert_eq!(manifest.warnings.len(), 1);
    }

    #[test]
    fn archive_meta_maps_core_identifier_as_a_field() {
        let tables = build_tables(&request(BundleFormat::DarwinCoreArchive)).tables;
        let meta = meta_xml(&tables.all());
        assert!(meta.contains("<id index=\"0\"/>"));
        assert!(
            meta.contains(
                "<field index=\"0\" term=\"http://rs.tdwg.org/dwc/terms/occurrenceID\"/>"
            )
        );
    }
}
