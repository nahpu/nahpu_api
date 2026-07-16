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

const DWC_DP_PROFILE: &str = "http://rs.tdwg.org/dwc-dp/1.0/dwc-dp-profile.json";
const DWC_TERMS: &str = "http://rs.tdwg.org/dwc/terms/";
const DCTERMS: &str = "http://purl.org/dc/terms/";
const AC_TERMS: &str = "http://rs.tdwg.org/ac/terms/";

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

#[derive(Clone, Debug)]
struct Table {
    name: &'static str,
    row_type: &'static str,
    core: bool,
    headers: Vec<String>,
    rows: Vec<BTreeMap<String, String>>,
}

/// Returns a deterministic file manifest without creating a bundle.
pub fn plan_bundle_json(input_json: &str) -> Result<String, String> {
    let request: BundleRequest =
        serde_json::from_str(input_json).map_err(|error| error.to_string())?;
    let manifest = build_manifest(&request)?;
    serde_json::to_string(&manifest).map_err(|error| error.to_string())
}

/// Writes a standards-shaped Darwin Core bundle. Archives are ZIP files; Data Packages are directories.
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
    let tables = build_tables(&request);
    errors.extend(validate_relationships(&tables));
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
    let tables = build_tables(request);
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
    for table in tables.all() {
        files.push(BundleFile {
            path: format!("{}.csv", table.name),
            media_type: "text/csv".to_string(),
            records: table.rows.len(),
            columns: table.headers.clone(),
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
    let mut warnings = media_warnings(&request.media);
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
        let tables = build_tables(request);
        write_tables(&staging, &tables.all())?;
        fs::write(staging.join("meta.xml"), meta_xml(&tables.all())).map_err(io_error)?;
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
        let tables = build_tables(request);
        write_tables(&staging, &tables.all())?;
        fs::write(staging.join("eml.xml"), eml_xml(request)).map_err(io_error)?;
        let descriptor = data_package_json(request, &tables.all());
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
    events: Option<Table>,
    materials: Option<Table>,
    measurements: Option<Table>,
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
            &self.events,
            &self.materials,
            &self.measurements,
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

fn build_tables(request: &BundleRequest) -> Tables {
    let occurrence_rows = normalize_rows(&request.occurrences);
    let material_rows = normalize_rows(&request.materials);
    let measurement_rows = normalize_rows(&request.measurements);
    let raw_media_rows = request
        .media
        .iter()
        .map(|media| media.fields.clone())
        .collect::<Vec<_>>();
    let media_rows = normalize_rows(&raw_media_rows);

    if request.format == BundleFormat::DarwinCoreDataPackage {
        let occurrences = occurrence_rows
            .into_iter()
            .map(|mut row| {
                copy_key(&mut row, "occurrenceID", "occurrence_pk");
                copy_key(&mut row, "eventID", "event_fk");
                row.entry("occurrenceStatus".to_string())
                    .or_insert_with(|| "detected".to_string());
                row
            })
            .collect();
        let events = normalize_rows(&request.events)
            .into_iter()
            .map(|mut row| {
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
                row.remove("occurrenceID");
                row
            })
            .collect();
        let assertions = measurement_rows
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
            occurrences: table(
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
            events: optional_table(
                "event",
                "https://rs.tdwg.org/dwc-dp/terms/Event",
                events,
                &["event_pk", "eventID", "eventCategory"],
            ),
            materials: optional_table(
                "material",
                "http://rs.tdwg.org/dwc/terms/MaterialEntity",
                materials,
                &["materialEntity_pk", "materialEntityID"],
            ),
            measurements: optional_table(
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
            media: optional_table(
                "media",
                "http://rs.tdwg.org/ac/terms/Media",
                media,
                &["media_pk", "mediaID"],
            ),
            agents: optional_table(
                "agent",
                "http://purl.org/dc/terms/Agent",
                agents,
                &["agent_pk", "agentID", "agentType", "preferredAgentName"],
            ),
            occurrence_agent_roles: role_table(
                "occurrence-agent-role",
                normalize_rows(&request.occurrence_agent_roles),
                "occurrenceID",
                "occurrence_fk",
            ),
            event_agent_roles: role_table(
                "event-agent-role",
                normalize_rows(&request.event_agent_roles),
                "eventID",
                "event_fk",
            ),
            material_agent_roles: role_table(
                "material-agent-role",
                normalize_rows(&request.material_agent_roles),
                "materialEntityID",
                "materialEntity_fk",
            ),
            media_agent_roles: role_table(
                "media-agent-role",
                normalize_rows(&request.media_agent_roles),
                "mediaID",
                "media_fk",
            ),
            occurrence_media: optional_table(
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
    let occurrences = table(
        "occurrence",
        "http://rs.tdwg.org/dwc/terms/Occurrence",
        true,
        occurrence_rows,
        &["occurrenceID", "basisOfRecord"],
    );
    let materials = optional_table(
        "material",
        "http://rs.tdwg.org/dwc/terms/MaterialEntity",
        material_rows,
        &["occurrenceID"],
    );
    let measurements = optional_table(
        "measurement_or_fact",
        "http://rs.tdwg.org/dwc/terms/MeasurementOrFact",
        measurement_rows,
        &["occurrenceID"],
    );
    let media = optional_table(
        "multimedia",
        "http://rs.gbif.org/terms/1.0/Multimedia",
        media_rows,
        &["occurrenceID"],
    );
    Tables {
        occurrences,
        events: None,
        materials,
        measurements,
        media,
        agents: None,
        occurrence_agent_roles: None,
        event_agent_roles: None,
        material_agent_roles: None,
        media_agent_roles: None,
        occurrence_media: None,
    }
}

fn role_table(
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
    optional_table(
        name,
        "http://rs.tdwg.org/dwc/terms/AgentRole",
        rows,
        &[target_key, "agent_fk", "agentRole", "agentRoleOrder"],
    )
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
    errors
}

fn key_values(table: &Table, key: &str) -> BTreeSet<String> {
    table
        .rows
        .iter()
        .filter_map(|row| row.get(key).cloned())
        .collect()
}

fn optional_table(
    name: &'static str,
    row_type: &'static str,
    rows: Vec<BTreeMap<String, String>>,
    required: &[&str],
) -> Option<Table> {
    (!rows.is_empty()).then(|| table(name, row_type, false, rows, required))
}

fn table(
    name: &'static str,
    row_type: &'static str,
    core: bool,
    rows: Vec<BTreeMap<String, String>>,
    required: &[&str],
) -> Table {
    let mut headers = required
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    let mut optional = BTreeSet::new();
    for row in &rows {
        for (key, value) in row {
            if !value.is_empty() && !required.contains(&key.as_str()) {
                optional.insert(key.clone());
            }
        }
    }
    headers.extend(optional);
    Table {
        name,
        row_type,
        core,
        headers,
        rows,
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
            .write_record(&table.headers)
            .map_err(|error| error.to_string())?;
        for row in &table.rows {
            let record = table
                .headers
                .iter()
                .map(|header| row.get(header).map_or("", String::as_str));
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
    for (index, header) in table.headers.iter().enumerate().skip(first_field) {
        xml.push_str(&format!(
            "    <field index=\"{index}\" term=\"{}\"/>\n",
            term_uri(header)
        ));
    }
    xml.push_str(&format!("  </{element}>\n"));
    xml
}

fn term_uri(header: &str) -> String {
    if let Some(term) = header.strip_prefix("dcterms:") {
        format!("{DCTERMS}{term}")
    } else if let Some(term) = header.strip_prefix("ac:") {
        format!("{AC_TERMS}{term}")
    } else if header.starts_with("http://") || header.starts_with("https://") {
        header.to_string()
    } else {
        format!("{DWC_TERMS}{header}")
    }
}

fn data_package_json(request: &BundleRequest, tables: &[Table]) -> Value {
    let resources = tables
        .iter()
        .map(|table| {
            let fields = table
                .headers
                .iter()
                .map(|header| field_descriptor(table.name, header))
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
        "occurrence" => Some("occurrence_pk"),
        "material" => Some("materialEntity_pk"),
        "agent" => Some("agent_pk"),
        "media" => Some("media_pk"),
        _ => None,
    }
}

fn foreign_keys(table_name: &str) -> Vec<Value> {
    let relationships: &[(&str, &str, &str, &str)] = match table_name {
        "occurrence" => &[("event_fk", "happened during", "event", "event_pk")],
        "material" => &[(
            "collectionEvent_fk",
            "collected during",
            "event",
            "event_pk",
        )],
        "occurrence-assertion" => &[("occurrence_fk", "about", "occurrence", "occurrence_pk")],
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

fn field_descriptor(table_name: &str, field_name: &str) -> Value {
    let title = field_title(field_name);
    let (description, term, field_type) = descriptor_details(table_name, field_name);
    serde_json::json!({
        "name": field_name,
        "title": title,
        "description": description,
        "type": field_type,
        "format": "default",
        "dcterms:isVersionOf": term,
    })
}

fn descriptor_details(table_name: &str, field_name: &str) -> (String, String, &'static str) {
    let field_type = if field_name == "agentRoleOrder" {
        "integer"
    } else {
        "string"
    };
    let term = match field_name {
        "preferredAgentName" | "title" => format!("{DCTERMS}title"),
        "mediaID" | "media_fk" => format!("{DCTERMS}identifier"),
        "mediaType" => format!("{DCTERMS}type"),
        "accessURI" => format!("{AC_TERMS}accessURI"),
        "agent_pk" | "agent_fk" | "agentID" => format!("{DWC_TERMS}agentID"),
        "event_pk" | "event_fk" => format!("{DWC_TERMS}eventID"),
        "occurrence_pk" | "occurrence_fk" => format!("{DWC_TERMS}occurrenceID"),
        "materialEntity_pk" | "materialEntity_fk" => {
            format!("{DWC_TERMS}materialEntityID")
        }
        "collectionEvent_fk" => format!("{DWC_TERMS}eventID"),
        "agentRole" => format!("{DWC_TERMS}relationshipOfResource"),
        "agentRoleOrder" => format!("{DWC_TERMS}agentRoleOrder"),
        "assertionID" => format!("{DWC_TERMS}assertionID"),
        "assertionType" => format!("{DWC_TERMS}assertionType"),
        "assertionValue" => format!("{DWC_TERMS}assertionValue"),
        "assertionUnit" => format!("{DWC_TERMS}assertionUnit"),
        value => format!("{DWC_TERMS}{value}"),
    };
    (
        format!(
            "{} field in the {table_name} table.",
            field_title(field_name)
        ),
        term,
        field_type,
    )
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
                ("empty".to_string(), Value::String(String::new())),
            ])],
            events: Vec::new(),
            materials: Vec::new(),
            measurements: Vec::new(),
            media: Vec::new(),
            agents: Vec::new(),
            occurrence_agent_roles: Vec::new(),
            event_agent_roles: Vec::new(),
            material_agent_roles: Vec::new(),
            media_agent_roles: Vec::new(),
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
        assert!(!occurrence.columns.contains(&"empty".to_string()));
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

        let tables = build_tables(&request);
        let row = &tables.occurrences.rows[0];
        assert_eq!(row["occurrenceID"], "occ-1");
        assert_eq!(row["occurrence_pk"], "occ-1");
    }

    #[test]
    fn data_package_uses_official_profile_and_complete_field_descriptors() {
        let request = request(BundleFormat::DarwinCoreDataPackage);
        let tables = build_tables(&request);
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
        let tables = build_tables(&request(BundleFormat::DarwinCoreArchive));
        let meta = meta_xml(&tables.all());
        assert!(meta.contains("<id index=\"0\"/>"));
        assert!(
            meta.contains(
                "<field index=\"0\" term=\"http://rs.tdwg.org/dwc/terms/occurrenceID\"/>"
            )
        );
    }
}
