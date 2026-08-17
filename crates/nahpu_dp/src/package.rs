use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use nahpu_archive::{
    tar_gzip::{TarGzipArchive, TarGzipExtractor},
    zip::{ZipArchive, ZipExtractor},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::FORMAT_VERSION;

pub const REQUIRED_NAHPU_TABLES: [&str; 4] = ["project", "site", "collEvent", "specimen"];

const PROJECT_JSON_ONLY_COLLECTIONS: [&str; 1] = ["personnelPhoto"];

const PROJECT_PATH: &str = "nahpu-project.json";
const ENUM_MAPPING_PATH: &str = "mappings/sqlite_enums.csv";
const VOCABULARY_SECTIONS: [&str; 4] = ["site", "events", "specimens", "parasites"];
const REQUIRED_VOCABULARIES: [(&str, &str); 6] = [
    ("site", "siteTypes"),
    ("site", "habitatTypes"),
    ("events", "collEventMethods"),
    ("events", "collPersonnelRoles"),
    ("specimens", "specimenTypes"),
    ("specimens", "specimenTreatment"),
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat {
    TarGzip,
    Zip,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PackageColumn {
    pub name: String,
    pub data_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub primary_key: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PackageForeignKey {
    pub fields: String,
    pub resource: String,
    pub reference_fields: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PackageTable {
    pub name: String,
    pub columns: Vec<PackageColumn>,
    #[serde(default)]
    pub foreign_keys: Vec<PackageForeignKey>,
    #[serde(default)]
    pub rows: Vec<BTreeMap<String, Value>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PackageFile {
    pub source_path: String,
    pub package_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnumMapping {
    pub table: String,
    pub column: String,
    pub enum_type: String,
    pub sqlite_index: i64,
    pub enum_name: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ControlledVocabulary {
    pub section: String,
    pub config_key: String,
    pub vocabulary_name: String,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PackageRequest {
    pub archive_format: ArchiveFormat,
    pub name: String,
    pub app_name: String,
    pub app_version: String,
    pub app_build: String,
    pub database_schema_version: u32,
    pub user_config_schema_version: u32,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    pub project_json: String,
    #[serde(default)]
    pub database_schema_path: Option<String>,
    pub user_configs: Value,
    pub tables: Vec<PackageTable>,
    #[serde(default)]
    pub enum_mappings: Vec<EnumMapping>,
    #[serde(default)]
    pub controlled_vocabularies: Vec<ControlledVocabulary>,
    #[serde(default)]
    pub files: Vec<PackageFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManifestFile {
    pub path: String,
    pub media_type: String,
    pub records: usize,
    pub columns: Vec<String>,
    pub bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PackageManifest {
    pub package_type: String,
    pub archive_format: ArchiveFormat,
    pub files: Vec<ManifestFile>,
    pub warnings: Vec<String>,
}

pub fn plan_package_json(input_json: &str) -> Result<String, String> {
    let request = parse_request(input_json)?;
    let manifest = build_manifest(&request);
    serde_json::to_string(&manifest).map_err(|error| error.to_string())
}

pub fn validate_package_json(input_json: &str) -> Result<String, String> {
    let request = parse_request(input_json)?;
    let errors = validate_request(&request);
    serde_json::to_string(&errors).map_err(|error| error.to_string())
}

pub fn write_package_json(input_json: &str, output_path: &str) -> Result<String, String> {
    let request = parse_request(input_json)?;
    let errors = validate_request(&request);
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let manifest = write_package(&request, Path::new(output_path))?;
    serde_json::to_string(&manifest).map_err(|error| error.to_string())
}

fn parse_request(input_json: &str) -> Result<PackageRequest, String> {
    let request: PackageRequest =
        serde_json::from_str(input_json).map_err(|error| error.to_string())?;
    parse_project_json(&request)?;
    Ok(request)
}

fn validate_request(request: &PackageRequest) -> Vec<String> {
    let mut errors = Vec::new();
    if request.name.trim().is_empty() {
        errors.push("Package name is required.".to_string());
    }
    validate_project_json(request, &mut errors);

    let mut names = BTreeSet::new();
    for table in &request.tables {
        if !is_valid_table_name(&table.name) {
            errors.push(format!("Invalid NAHPU table name: {}", table.name));
        }
        if !names.insert(table.name.as_str()) {
            errors.push(format!("Duplicate NAHPU table: {}", table.name));
        }
    }
    for required in REQUIRED_NAHPU_TABLES {
        if !names.contains(required) {
            errors.push(format!("Required NAHPU table is missing: {required}"));
        }
    }
    for table in &request.tables {
        if table.columns.is_empty() {
            errors.push(format!("Table {} has no column metadata.", table.name));
        }
        let columns = table
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<BTreeSet<_>>();
        for row in &table.rows {
            for key in row.keys() {
                if !columns.contains(key.as_str()) {
                    errors.push(format!(
                        "Table {} contains an undeclared column: {key}",
                        table.name
                    ));
                }
            }
        }
    }
    validate_enum_mappings(request, &mut errors);
    validate_controlled_vocabularies(request, &mut errors);
    for file in &request.files {
        if !Path::new(&file.source_path).is_file() {
            continue;
        }
        if validate_package_path(Path::new(&file.package_path)).is_err() {
            errors.push(format!("Unsafe package file path: {}", file.package_path));
        } else if is_sqlite_path(&file.package_path) {
            errors.push(format!(
                "SQLite files are not allowed in a NAHPU Data Package: {}",
                file.package_path
            ));
        }
    }
    errors
}

fn build_manifest(request: &PackageRequest) -> PackageManifest {
    let mut files = vec![
        metadata_file("datapackage.json", "application/json"),
        metadata_file("nahpu.toml", "application/toml"),
        ManifestFile {
            path: PROJECT_PATH.to_string(),
            media_type: "application/json".to_string(),
            records: project_record_count(request),
            columns: Vec::new(),
            bytes: None,
        },
        metadata_file("configs/user_configs.json", "application/json"),
        ManifestFile {
            path: ENUM_MAPPING_PATH.to_string(),
            media_type: "text/csv".to_string(),
            records: request.enum_mappings.len(),
            columns: enum_mapping_columns(),
            bytes: None,
        },
    ];
    for section in VOCABULARY_SECTIONS {
        let records = request
            .controlled_vocabularies
            .iter()
            .filter(|vocabulary| vocabulary.section == section)
            .map(|vocabulary| vocabulary.values.len())
            .sum();
        files.push(ManifestFile {
            path: vocabulary_path(section),
            media_type: "text/csv".to_string(),
            records,
            columns: controlled_vocabulary_columns(),
            bytes: None,
        });
    }
    if request.database_schema_path.is_some() {
        files.push(metadata_file("schemas/tables.drift", "text/plain"));
    }
    for table in &request.tables {
        files.push(ManifestFile {
            path: format!("tables/{}.csv", table.name),
            media_type: "text/csv".to_string(),
            records: table.rows.len(),
            columns: table
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect(),
            bytes: None,
        });
    }
    let mut warnings = project_warnings(request);
    for file in &request.files {
        if Path::new(&file.source_path).is_file() {
            files.push(metadata_file(
                &file.package_path,
                media_type_for_path(&file.package_path),
            ));
        } else {
            warnings.push(format!(
                "Referenced file was not found and was not bundled: {}",
                file.source_path
            ));
        }
    }
    PackageManifest {
        package_type: "nahpu_data_package".to_string(),
        archive_format: request.archive_format.clone(),
        files,
        warnings,
    }
}

fn write_package(request: &PackageRequest, output_path: &Path) -> Result<PackageManifest, String> {
    if output_path.exists() {
        return Err(format!(
            "Package destination already exists: {}",
            output_path.display()
        ));
    }
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(io_error)?;
    let staging = temporary_directory(parent, "nahpu-dp")?;
    let mut manifest = build_manifest(request);
    let result: Result<(), String> = (|| {
        fs::create_dir_all(staging.join("tables")).map_err(io_error)?;
        fs::create_dir_all(staging.join("configs")).map_err(io_error)?;
        fs::create_dir_all(staging.join("mappings")).map_err(io_error)?;
        fs::create_dir_all(staging.join("vocabularies")).map_err(io_error)?;
        fs::write(staging.join(PROJECT_PATH), &request.project_json).map_err(io_error)?;

        let user_configs =
            serde_json::to_vec_pretty(&request.user_configs).map_err(|error| error.to_string())?;
        fs::write(staging.join("configs/user_configs.json"), user_configs).map_err(io_error)?;

        if let Some(schema_path) = &request.database_schema_path
            && Path::new(schema_path).is_file()
        {
            fs::create_dir_all(staging.join("schemas")).map_err(io_error)?;
            fs::copy(schema_path, staging.join("schemas/tables.drift")).map_err(io_error)?;
        }

        for table in &request.tables {
            write_table(&staging, table)?;
        }
        write_enum_mappings(&staging, &request.enum_mappings)?;
        write_controlled_vocabularies(&staging, &request.controlled_vocabularies)?;
        copy_package_files(&staging, &request.files)?;

        fs::write(staging.join("nahpu.toml"), nahpu_toml(request)).map_err(io_error)?;
        let descriptor = data_package_json(request);
        fs::write(
            staging.join("datapackage.json"),
            serde_json::to_vec_pretty(&descriptor).map_err(|error| error.to_string())?,
        )
        .map_err(io_error)?;

        for file in &mut manifest.files {
            file.bytes = fs::metadata(staging.join(&file.path))
                .ok()
                .map(|metadata| metadata.len());
        }

        let files = collect_files(&staging)?;
        match request.archive_format {
            ArchiveFormat::TarGzip => TarGzipArchive::new(&staging, output_path, &files)
                .write()
                .map_err(io_error)?,
            ArchiveFormat::Zip => ZipArchive::new(&staging, None, output_path, &files)
                .write()
                .map_err(io_error)?,
        }
        verify_archive(request.archive_format.clone(), output_path, parent)?;
        Ok(())
    })();
    let _ = fs::remove_dir_all(&staging);
    if result.is_err() {
        let _ = fs::remove_file(output_path);
    }
    result?;
    Ok(manifest)
}

fn write_table(staging: &Path, table: &PackageTable) -> Result<(), String> {
    let output = staging.join(format!("tables/{}.csv", table.name));
    let mut writer = csv::Writer::from_path(output).map_err(|error| error.to_string())?;
    let headers = table
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    writer
        .write_record(&headers)
        .map_err(|error| error.to_string())?;
    for row in &table.rows {
        let values = table.columns.iter().map(|column| {
            row.get(&column.name)
                .map(value_to_string)
                .unwrap_or_default()
        });
        writer
            .write_record(values)
            .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}

fn write_enum_mappings(staging: &Path, mappings: &[EnumMapping]) -> Result<(), String> {
    let output = staging.join(ENUM_MAPPING_PATH);
    let mut writer = csv::Writer::from_path(output).map_err(|error| error.to_string())?;
    writer
        .write_record(enum_mapping_columns())
        .map_err(|error| error.to_string())?;
    for mapping in mappings {
        writer
            .serialize((
                &mapping.table,
                &mapping.column,
                &mapping.enum_type,
                mapping.sqlite_index,
                &mapping.enum_name,
                &mapping.display_name,
            ))
            .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}

fn write_controlled_vocabularies(
    staging: &Path,
    vocabularies: &[ControlledVocabulary],
) -> Result<(), String> {
    for section in VOCABULARY_SECTIONS {
        let output = staging.join(vocabulary_path(section));
        let mut writer = csv::Writer::from_path(output).map_err(|error| error.to_string())?;
        writer
            .write_record(controlled_vocabulary_columns())
            .map_err(|error| error.to_string())?;
        for vocabulary in vocabularies
            .iter()
            .filter(|vocabulary| vocabulary.section == section)
        {
            for (list_index, value) in vocabulary.values.iter().enumerate() {
                writer
                    .serialize((
                        &vocabulary.config_key,
                        &vocabulary.vocabulary_name,
                        list_index,
                        value,
                    ))
                    .map_err(|error| error.to_string())?;
            }
        }
        writer.flush().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn data_package_json(request: &PackageRequest) -> Value {
    let mut resources = request
        .tables
        .iter()
        .map(table_resource)
        .collect::<Vec<_>>();
    resources.extend([
        enum_mapping_resource(),
        controlled_vocabulary_resource("site"),
        controlled_vocabulary_resource("events"),
        controlled_vocabulary_resource("specimens"),
        controlled_vocabulary_resource("parasites"),
        simple_resource("nahpu-project", PROJECT_PATH, "json", "application/json"),
        simple_resource(
            "user-configs",
            "configs/user_configs.json",
            "json",
            "application/json",
        ),
        simple_resource("nahpu-manifest", "nahpu.toml", "toml", "application/toml"),
    ]);
    if request.database_schema_path.is_some() {
        resources.push(simple_resource(
            "database-schema",
            "schemas/tables.drift",
            "drift",
            "text/plain",
        ));
    }
    for (index, file) in request.files.iter().enumerate() {
        if Path::new(&file.source_path).is_file() {
            resources.push(simple_resource(
                &format!("file-{index}"),
                &file.package_path,
                Path::new(&file.package_path)
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("bin"),
                media_type_for_path(&file.package_path),
            ));
        }
    }
    serde_json::json!({
        "profile": "data-package",
        "name": package_name(&request.name),
        "title": request.name,
        "created": Utc::now().to_rfc3339(),
        "version": FORMAT_VERSION,
        "resources": resources,
        "nahpu": {
            "format": "NAHPU Data Package",
            "formatVersion": FORMAT_VERSION,
            "manifest": "nahpu.toml",
        },
    })
}

fn table_resource(table: &PackageTable) -> Value {
    let fields = table
        .columns
        .iter()
        .map(|column| {
            let mut field = serde_json::json!({
                "name": column.name,
                "type": frictionless_type(&column.data_type),
            });
            if column.required {
                field["constraints"] = serde_json::json!({"required": true});
            }
            field
        })
        .collect::<Vec<_>>();
    let primary_key = table
        .columns
        .iter()
        .filter(|column| column.primary_key)
        .map(|column| Value::String(column.name.clone()))
        .collect::<Vec<_>>();
    let foreign_keys = table
        .foreign_keys
        .iter()
        .map(|foreign_key| {
            serde_json::json!({
                "fields": foreign_key.fields,
                "reference": {
                    "resource": package_name(&foreign_key.resource),
                    "fields": foreign_key.reference_fields,
                },
            })
        })
        .collect::<Vec<_>>();
    let mut schema = serde_json::json!({"fields": fields, "missingValues": [""]});
    if !primary_key.is_empty() {
        schema["primaryKey"] = Value::Array(primary_key);
    }
    if !foreign_keys.is_empty() {
        schema["foreignKeys"] = Value::Array(foreign_keys);
    }
    serde_json::json!({
        "name": package_name(&table.name),
        "path": format!("tables/{}.csv", table.name),
        "profile": "tabular-data-resource",
        "format": "csv",
        "mediatype": "text/csv",
        "encoding": "utf-8",
        "schema": schema,
    })
}

fn enum_mapping_resource() -> Value {
    tabular_resource(
        "sqlite-enum-mappings",
        ENUM_MAPPING_PATH,
        vec![
            field_descriptor("table", "string"),
            field_descriptor("column", "string"),
            field_descriptor("enum_type", "string"),
            field_descriptor("sqlite_index", "integer"),
            field_descriptor("enum_name", "string"),
            field_descriptor("display_name", "string"),
        ],
        vec!["table", "column", "sqlite_index"],
    )
}

fn controlled_vocabulary_resource(section: &str) -> Value {
    tabular_resource(
        &format!("{section}-controlled-vocabularies"),
        &vocabulary_path(section),
        vec![
            field_descriptor("config_key", "string"),
            field_descriptor("vocabulary_name", "string"),
            field_descriptor("list_index", "integer"),
            field_descriptor("value", "string"),
        ],
        vec!["config_key", "list_index"],
    )
}

fn tabular_resource(name: &str, path: &str, fields: Vec<Value>, primary_key: Vec<&str>) -> Value {
    serde_json::json!({
        "name": package_name(name),
        "path": path,
        "profile": "tabular-data-resource",
        "format": "csv",
        "mediatype": "text/csv",
        "encoding": "utf-8",
        "schema": {
            "fields": fields,
            "primaryKey": primary_key,
            "missingValues": [""],
        },
    })
}

fn field_descriptor(name: &str, field_type: &str) -> Value {
    serde_json::json!({
        "name": name,
        "type": field_type,
        "constraints": {"required": true},
    })
}

fn simple_resource(name: &str, path: &str, format: &str, media_type: &str) -> Value {
    serde_json::json!({
        "name": package_name(name),
        "path": path,
        "format": format,
        "mediatype": media_type,
    })
}

fn nahpu_toml(request: &PackageRequest) -> String {
    let mut output = format!(
        "format_name = \"NAHPU Data Package\"\nformat_version = \"{}\"\ncreated_at = \"{}\"\n\n[application]\nname = \"{}\"\nversion = \"{}\"\nbuild_number = \"{}\"\n\n[schemas]\ndatabase = {}\nuser_configs = {}\n\n[package]\ndescriptor = \"datapackage.json\"\nproject = \"{}\"\nuser_configs = \"configs/user_configs.json\"\nenum_mappings = \"{}\"\ntable_count = {}\nenum_mapping_count = {}\ncontrolled_vocabulary_count = {}\n\n[package.controlled_vocabularies]\nsite = \"{}\"\nevents = \"{}\"\nspecimens = \"{}\"\nparasites = \"{}\"\n\n[nahpu_api.dependencies]\n",
        FORMAT_VERSION,
        Utc::now().to_rfc3339(),
        toml_escape(&request.app_name),
        toml_escape(&request.app_version),
        toml_escape(&request.app_build),
        request.database_schema_version,
        request.user_config_schema_version,
        PROJECT_PATH,
        ENUM_MAPPING_PATH,
        request.tables.len(),
        request.enum_mappings.len(),
        request.controlled_vocabularies.len(),
        vocabulary_path("site"),
        vocabulary_path("events"),
        vocabulary_path("specimens"),
        vocabulary_path("parasites"),
    );
    for (name, version) in &request.dependencies {
        output.push_str(&format!(
            "\"{}\" = \"{}\"\n",
            toml_escape(name),
            toml_escape(version)
        ));
    }
    output
}

fn copy_package_files(staging: &Path, files: &[PackageFile]) -> Result<(), String> {
    let mut destinations = BTreeSet::new();
    for file in files {
        let source = Path::new(&file.source_path);
        if !source.is_file() {
            continue;
        }
        let relative = Path::new(&file.package_path);
        validate_package_path(relative).map_err(io_error)?;
        if !destinations.insert(file.package_path.clone()) {
            return Err(format!("Duplicate package path: {}", file.package_path));
        }
        let destination = staging.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        fs::copy(source, destination).map_err(io_error)?;
    }
    Ok(())
}

fn verify_archive(format: ArchiveFormat, path: &Path, parent: &Path) -> Result<(), String> {
    let output = temporary_directory(parent, "nahpu-dp-verify")?;
    let result = match format {
        ArchiveFormat::TarGzip => TarGzipExtractor::new(path, &output).extract(),
        ArchiveFormat::Zip => ZipExtractor::new(path, &output).extract(),
    };
    let descriptor_exists = output.join("datapackage.json").is_file();
    let manifest_exists = output.join("nahpu.toml").is_file();
    let project_exists = output.join(PROJECT_PATH).is_file();
    let mappings_exist = output.join(ENUM_MAPPING_PATH).is_file();
    let vocabularies_exist = VOCABULARY_SECTIONS
        .iter()
        .all(|section| output.join(vocabulary_path(section)).is_file());
    let _ = fs::remove_dir_all(&output);
    result.map_err(io_error)?;
    if !descriptor_exists
        || !manifest_exists
        || !project_exists
        || !mappings_exist
        || !vocabularies_exist
    {
        return Err("Package verification failed: required metadata is missing.".to_string());
    }
    Ok(())
}

fn parse_project_json(request: &PackageRequest) -> Result<Value, String> {
    serde_json::from_str(&request.project_json)
        .map_err(|error| format!("Invalid NAHPU project JSON: {error}"))
}

fn validate_project_json(request: &PackageRequest, errors: &mut Vec<String>) {
    let Ok(project_json) = parse_project_json(request) else {
        errors.push("NAHPU project JSON is invalid.".to_string());
        return;
    };
    let Some(object) = project_json.as_object() else {
        errors.push("NAHPU project JSON must be an object.".to_string());
        return;
    };
    if object.get("nahpu_project").and_then(Value::as_str) != Some("project") {
        errors.push("NAHPU project JSON has an invalid project marker.".to_string());
    }
    if !object.get("version").is_some_and(Value::is_number) {
        errors.push("NAHPU project JSON is missing its format version.".to_string());
    }
    let Some(project) = object.get("project").and_then(Value::as_object) else {
        errors.push("NAHPU project JSON is missing project information.".to_string());
        return;
    };
    let Some(records) = object.get("records").and_then(Value::as_object) else {
        errors.push("NAHPU project JSON is missing project records.".to_string());
        return;
    };

    if let (Some(weather), Some(environment)) = (records.get("weather"), records.get("environment"))
        && weather != environment
    {
        errors.push(
            "NAHPU project JSON contains conflicting weather and environment records.".to_string(),
        );
    }

    for table in &request.tables {
        let expected = if table.name == "project" {
            Value::Array(vec![Value::Object(project.clone())])
        } else {
            project_records_for_table(records, &table.name)
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new()))
        };
        let actual = serde_json::to_value(&table.rows).unwrap_or(Value::Null);
        if actual != expected {
            errors.push(format!(
                "Table {} does not match nahpu-project.json.",
                table.name
            ));
        }
    }

    for (name, value) in records {
        let Some(rows) = value.as_array() else {
            continue;
        };
        if rows.is_empty() || PROJECT_JSON_ONLY_COLLECTIONS.contains(&name.as_str()) {
            continue;
        }
        if !request
            .tables
            .iter()
            .any(|table| table_matches_project_collection(&table.name, name))
        {
            errors.push(format!(
                "Project JSON contains data for table {name}, but no table resource was provided."
            ));
        }
    }
}

fn project_records_for_table<'a>(
    records: &'a serde_json::Map<String, Value>,
    table_name: &str,
) -> Option<&'a Value> {
    records.get(table_name).or_else(|| match table_name {
        "weather" => records.get("environment"),
        "environment" => records.get("weather"),
        _ => None,
    })
}

fn table_matches_project_collection(table_name: &str, collection_name: &str) -> bool {
    table_name == collection_name
        || matches!(
            (table_name, collection_name),
            ("weather", "environment") | ("environment", "weather")
        )
}

fn is_valid_table_name(name: &str) -> bool {
    let mut characters = name.bytes();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == b'_')
}

fn project_record_count(request: &PackageRequest) -> usize {
    let Ok(project_json) = parse_project_json(request) else {
        return 0;
    };
    let records = project_json
        .get("records")
        .and_then(Value::as_object)
        .map(|records| {
            records
                .values()
                .filter_map(Value::as_array)
                .map(Vec::len)
                .sum::<usize>()
        })
        .unwrap_or_default();
    records + usize::from(project_json.get("project").is_some_and(Value::is_object))
}

fn project_warnings(request: &PackageRequest) -> Vec<String> {
    parse_project_json(request)
        .ok()
        .and_then(|project| project.get("warnings").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|warning| warning.as_str().map(ToString::to_string))
        .collect()
}

fn is_sqlite_path(package_path: &str) -> bool {
    Path::new(package_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "db" | "sqlite" | "sqlite3"
            )
        })
}

fn validate_enum_mappings(request: &PackageRequest, errors: &mut Vec<String>) {
    if request.enum_mappings.is_empty() {
        errors.push("SQLite enum mappings are required.".to_string());
        return;
    }
    let mut keys = BTreeSet::new();
    let mut names = BTreeSet::new();
    let table_columns = request
        .tables
        .iter()
        .map(|table| {
            (
                table.name.as_str(),
                table
                    .columns
                    .iter()
                    .map(|column| column.name.as_str())
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for mapping in &request.enum_mappings {
        if mapping.table.trim().is_empty()
            || mapping.column.trim().is_empty()
            || mapping.enum_type.trim().is_empty()
            || mapping.enum_name.trim().is_empty()
            || mapping.display_name.trim().is_empty()
        {
            errors.push("SQLite enum mappings must not contain empty fields.".to_string());
        }
        if mapping.sqlite_index < 0 {
            errors.push(format!(
                "SQLite enum index must not be negative: {}.{} index {}",
                mapping.table, mapping.column, mapping.sqlite_index
            ));
        }
        match table_columns.get(mapping.table.as_str()) {
            Some(columns) if !columns.contains(mapping.column.as_str()) => errors.push(format!(
                "SQLite enum mapping references an unknown column: {}.{}",
                mapping.table, mapping.column
            )),
            None => errors.push(format!(
                "SQLite enum mapping references an unknown table: {}",
                mapping.table
            )),
            Some(_) => {}
        }
        let key = (
            mapping.table.as_str(),
            mapping.column.as_str(),
            mapping.sqlite_index,
        );
        if !keys.insert(key) {
            errors.push(format!(
                "Duplicate SQLite enum mapping: {}.{} index {}",
                mapping.table, mapping.column, mapping.sqlite_index
            ));
        }
        let name = (
            mapping.table.as_str(),
            mapping.column.as_str(),
            mapping.enum_name.as_str(),
        );
        if !names.insert(name) {
            errors.push(format!(
                "Duplicate SQLite enum name: {}.{} {}",
                mapping.table, mapping.column, mapping.enum_name
            ));
        }
    }
}

fn validate_controlled_vocabularies(request: &PackageRequest, errors: &mut Vec<String>) {
    let mut keys = BTreeSet::new();
    for vocabulary in &request.controlled_vocabularies {
        let required = REQUIRED_VOCABULARIES
            .iter()
            .any(|(_, config_key)| *config_key == vocabulary.config_key);
        if !VOCABULARY_SECTIONS.contains(&vocabulary.section.as_str()) {
            errors.push(format!(
                "Unknown controlled vocabulary section: {}",
                vocabulary.section
            ));
        }
        if vocabulary.config_key.trim().is_empty()
            || vocabulary.vocabulary_name.trim().is_empty()
            || (required && vocabulary.values.is_empty())
            || vocabulary
                .values
                .iter()
                .any(|value| value.trim().is_empty())
        {
            errors.push(format!(
                "Controlled vocabulary {} must have a name and non-empty values.",
                vocabulary.config_key
            ));
        }
        if !keys.insert(vocabulary.config_key.as_str()) {
            errors.push(format!(
                "Duplicate controlled vocabulary: {}",
                vocabulary.config_key
            ));
        }
    }
    for (section, config_key) in REQUIRED_VOCABULARIES {
        match request
            .controlled_vocabularies
            .iter()
            .find(|vocabulary| vocabulary.config_key == config_key)
        {
            Some(vocabulary) if vocabulary.section != section => errors.push(format!(
                "Controlled vocabulary {config_key} must use section {section}."
            )),
            None => errors.push(format!(
                "Required controlled vocabulary is missing: {config_key}"
            )),
            Some(_) => {}
        }
    }
}

fn validate_package_path(path: &Path) -> io::Result<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsafe package path: {}", path.display()),
        ));
    }
    Ok(())
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        _ => value.to_string(),
    }
}

fn frictionless_type(data_type: &str) -> &'static str {
    let normalized = data_type.to_ascii_uppercase();
    if normalized.contains("BOOL") {
        "boolean"
    } else if normalized.contains("INT") {
        "integer"
    } else if normalized.contains("REAL")
        || normalized.contains("FLOA")
        || normalized.contains("DOUB")
        || normalized.contains("NUM")
    {
        "number"
    } else {
        "string"
    }
}

fn package_name(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn metadata_file(path: &str, media_type: &str) -> ManifestFile {
    ManifestFile {
        path: path.to_string(),
        media_type: media_type.to_string(),
        records: 0,
        columns: Vec::new(),
        bytes: None,
    }
}

fn enum_mapping_columns() -> Vec<String> {
    [
        "table",
        "column",
        "enum_type",
        "sqlite_index",
        "enum_name",
        "display_name",
    ]
    .map(str::to_string)
    .to_vec()
}

fn controlled_vocabulary_columns() -> Vec<String> {
    ["config_key", "vocabulary_name", "list_index", "value"]
        .map(str::to_string)
        .to_vec()
}

fn vocabulary_path(section: &str) -> String {
    format!("vocabularies/{section}.csv")
}

fn media_type_for_path(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "tif" | "tiff" => "image/tiff",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "json" => "application/json",
        "toml" => "application/toml",
        "csv" => "text/csv",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    }
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

fn io_error(error: io::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn request(_directory: &Path, archive_format: ArchiveFormat) -> PackageRequest {
        let tables = REQUIRED_NAHPU_TABLES
            .iter()
            .map(|name| {
                let mut columns = vec![PackageColumn {
                    name: "id".to_string(),
                    data_type: "INT".to_string(),
                    required: false,
                    primary_key: false,
                }];
                if *name == "specimen" {
                    columns.push(PackageColumn {
                        name: "iDConfidence".to_string(),
                        data_type: "INT".to_string(),
                        required: false,
                        primary_key: false,
                    });
                }
                PackageTable {
                    name: (*name).to_string(),
                    columns,
                    foreign_keys: Vec::new(),
                    rows: if *name == "project" {
                        vec![BTreeMap::from([(
                            "id".to_string(),
                            Value::String("project-a".to_string()),
                        )])]
                    } else {
                        Vec::new()
                    },
                }
            })
            .collect();
        let records = REQUIRED_NAHPU_TABLES
            .iter()
            .filter(|name| **name != "project")
            .map(|name| ((*name).to_string(), Value::Array(Vec::new())))
            .collect::<serde_json::Map<_, _>>();
        let project_json = serde_json::to_string_pretty(&serde_json::json!({
            "nahpu_project": "project",
            "version": 4,
            "project": {"id": "project-a"},
            "records": records,
            "media": [],
            "warnings": ["Fixture warning"],
        }))
        .unwrap();
        PackageRequest {
            archive_format,
            name: "NAHPU test".to_string(),
            app_name: "NAHPU".to_string(),
            app_version: "1.0.0".to_string(),
            app_build: "1".to_string(),
            database_schema_version: 7,
            user_config_schema_version: 1,
            dependencies: BTreeMap::from([("nahpu_dp".to_string(), "0.1.0".to_string())]),
            project_json,
            database_schema_path: None,
            user_configs: serde_json::json!({"schema_version": 1}),
            tables,
            enum_mappings: vec![
                EnumMapping {
                    table: "specimen".to_string(),
                    column: "iDConfidence".to_string(),
                    enum_type: "IdentificationConfidence".to_string(),
                    sqlite_index: 0,
                    enum_name: "low".to_string(),
                    display_name: "Low".to_string(),
                },
                EnumMapping {
                    table: "specimen".to_string(),
                    column: "iDConfidence".to_string(),
                    enum_type: "IdentificationConfidence".to_string(),
                    sqlite_index: 1,
                    enum_name: "medium".to_string(),
                    display_name: "Medium".to_string(),
                },
            ],
            controlled_vocabularies: vec![
                vocabulary("site", "siteTypes", "Site type"),
                vocabulary("site", "habitatTypes", "Habitat type"),
                vocabulary("events", "collEventMethods", "Collecting method"),
                vocabulary("events", "collPersonnelRoles", "Collecting personnel role"),
                vocabulary("specimens", "specimenTypes", "Specimen type"),
                vocabulary("specimens", "specimenTreatment", "Specimen treatment"),
            ],
            files: Vec::new(),
        }
    }

    fn vocabulary(section: &str, config_key: &str, name: &str) -> ControlledVocabulary {
        ControlledVocabulary {
            section: section.to_string(),
            config_key: config_key.to_string(),
            vocabulary_name: name.to_string(),
            values: vec![format!("{name} value")],
        }
    }

    #[test]
    fn validates_all_required_tables() {
        let directory = tempdir().unwrap();
        let mut request = request(directory.path(), ArchiveFormat::Zip);
        request.tables.pop();
        let errors = validate_request(&request);

        assert!(errors.contains(&"Required NAHPU table is missing: specimen".to_string()));
    }

    #[test]
    fn accepts_missing_optional_tables() {
        let directory = tempdir().unwrap();
        let request = request(directory.path(), ArchiveFormat::Zip);

        assert!(validate_request(&request).is_empty());
    }

    #[test]
    fn accepts_empty_optional_parasite_vocabulary() {
        let directory = tempdir().unwrap();
        let mut request = request(directory.path(), ArchiveFormat::Zip);
        request.controlled_vocabularies.push(ControlledVocabulary {
            section: "parasites".to_string(),
            config_key: "parasiteCategories".to_string(),
            vocabulary_name: "Parasite category".to_string(),
            values: Vec::new(),
        });

        assert!(validate_request(&request).is_empty());
    }

    #[test]
    fn accepts_optional_parasite_and_legacy_weather_tables() {
        let directory = tempdir().unwrap();
        let mut request = request(directory.path(), ArchiveFormat::Zip);
        let mut project_json: Value = serde_json::from_str(&request.project_json).unwrap();
        project_json["records"]["weather"] = serde_json::json!([{"id": 1}]);
        project_json["records"]["parasite"] = serde_json::json!([{"id": 2}]);
        request.project_json = serde_json::to_string(&project_json).unwrap();
        request.tables.extend([
            PackageTable {
                name: "weather".to_string(),
                columns: vec![PackageColumn {
                    name: "id".to_string(),
                    data_type: "INT".to_string(),
                    required: false,
                    primary_key: false,
                }],
                foreign_keys: Vec::new(),
                rows: vec![BTreeMap::from([("id".to_string(), Value::from(1))])],
            },
            PackageTable {
                name: "parasite".to_string(),
                columns: vec![PackageColumn {
                    name: "id".to_string(),
                    data_type: "INT".to_string(),
                    required: false,
                    primary_key: false,
                }],
                foreign_keys: Vec::new(),
                rows: vec![BTreeMap::from([("id".to_string(), Value::from(2))])],
            },
        ]);

        assert!(validate_request(&request).is_empty());
    }

    #[test]
    fn rejects_missing_table_resource_for_non_empty_project_records() {
        let directory = tempdir().unwrap();
        let mut request = request(directory.path(), ArchiveFormat::Zip);
        let mut project_json: Value = serde_json::from_str(&request.project_json).unwrap();
        project_json["records"]["parasite"] = serde_json::json!([{"id": 2}]);
        request.project_json = serde_json::to_string(&project_json).unwrap();

        let errors = validate_request(&request);

        assert!(
            errors
                .iter()
                .any(|error| { error.contains("Project JSON contains data for table parasite") })
        );
    }

    #[test]
    fn rejects_invalid_and_duplicate_table_names() {
        let directory = tempdir().unwrap();
        let mut request = request(directory.path(), ArchiveFormat::Zip);
        let duplicate = request.tables[1].clone();
        request.tables.push(duplicate);
        request.tables.push(PackageTable {
            name: "../unsafe".to_string(),
            columns: vec![PackageColumn {
                name: "id".to_string(),
                data_type: "INT".to_string(),
                required: false,
                primary_key: false,
            }],
            foreign_keys: Vec::new(),
            rows: Vec::new(),
        });

        let errors = validate_request(&request);

        assert!(
            errors
                .iter()
                .any(|error| error.contains("Duplicate NAHPU table"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("Invalid NAHPU table name"))
        );
    }

    #[test]
    fn validates_required_mapping_metadata() {
        let directory = tempdir().unwrap();
        let mut request = request(directory.path(), ArchiveFormat::Zip);
        request.enum_mappings.clear();
        request.controlled_vocabularies.pop();
        let errors = validate_request(&request);
        assert!(errors.contains(&"SQLite enum mappings are required.".to_string()));
        assert!(
            errors.contains(
                &"Required controlled vocabulary is missing: specimenTreatment".to_string()
            )
        );
    }

    #[test]
    fn rejects_tables_that_do_not_match_project_json() {
        let directory = tempdir().unwrap();
        let mut request = request(directory.path(), ArchiveFormat::Zip);
        request
            .tables
            .iter_mut()
            .find(|table| table.name == "site")
            .unwrap()
            .rows
            .push(BTreeMap::from([("id".to_string(), Value::from(1))]));

        let errors = validate_request(&request);

        assert!(errors.contains(&"Table site does not match nahpu-project.json.".to_string()));
    }

    #[test]
    fn rejects_sqlite_package_files() {
        let directory = tempdir().unwrap();
        let database = directory.path().join("legacy.sqlite3");
        fs::write(&database, b"sqlite fixture").unwrap();
        let mut request = request(directory.path(), ArchiveFormat::Zip);
        request.files.push(PackageFile {
            source_path: database.to_string_lossy().to_string(),
            package_path: "database/legacy.sqlite3".to_string(),
        });

        let errors = validate_request(&request);

        assert!(
            errors
                .iter()
                .any(|error| error.contains("SQLite files are not allowed"))
        );
    }

    #[test]
    fn writes_zip_and_tar_gzip_packages() {
        let directory = tempdir().unwrap();
        for format in [ArchiveFormat::Zip, ArchiveFormat::TarGzip] {
            let request = request(directory.path(), format.clone());
            let extension = match format {
                ArchiveFormat::Zip => "zip",
                ArchiveFormat::TarGzip => "tar.gz",
            };
            let output = directory.path().join(format!("package.{extension}"));
            write_package(&request, &output).unwrap();
            assert!(output.is_file());

            let extracted = directory.path().join(format!("extracted-{extension}"));
            match format {
                ArchiveFormat::Zip => ZipExtractor::new(&output, &extracted).extract().unwrap(),
                ArchiveFormat::TarGzip => TarGzipExtractor::new(&output, &extracted)
                    .extract()
                    .unwrap(),
            }
            assert!(extracted.join("datapackage.json").is_file());
            assert_eq!(
                fs::read_to_string(extracted.join(PROJECT_PATH)).unwrap(),
                request.project_json
            );
            assert!(!extracted.join("database/nahpu.sqlite3").exists());
            assert!(extracted.join("configs/user_configs.json").is_file());
            assert!(extracted.join(ENUM_MAPPING_PATH).is_file());
            for section in VOCABULARY_SECTIONS {
                assert!(extracted.join(vocabulary_path(section)).is_file());
            }
            for table in REQUIRED_NAHPU_TABLES {
                assert!(extracted.join(format!("tables/{table}.csv")).is_file());
            }
            let mappings = fs::read_to_string(extracted.join(ENUM_MAPPING_PATH)).unwrap();
            assert!(mappings.contains("sqlite_index,enum_name,display_name"));
            assert!(mappings.contains("IdentificationConfidence,0,low,Low"));
            let site_vocabulary =
                fs::read_to_string(extracted.join(vocabulary_path("site"))).unwrap();
            assert!(site_vocabulary.contains("siteTypes,Site type,0,Site type value"));
            let metadata = fs::read_to_string(extracted.join("nahpu.toml")).unwrap();
            assert!(metadata.contains("database = 7"));
            assert!(metadata.contains("project = \"nahpu-project.json\""));
            assert!(metadata.contains("user_configs = 1"));
            assert!(metadata.contains("enum_mappings = \"mappings/sqlite_enums.csv\""));
            assert!(metadata.contains("\"nahpu_dp\" = \"0.1.0\""));
            let descriptor: Value =
                serde_json::from_slice(&fs::read(extracted.join("datapackage.json")).unwrap())
                    .unwrap();
            let resource_paths = descriptor["resources"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|resource| resource["path"].as_str())
                .collect::<BTreeSet<_>>();
            assert!(resource_paths.contains(PROJECT_PATH));
            assert!(!resource_paths.contains("database/nahpu.sqlite3"));
            assert!(resource_paths.contains(ENUM_MAPPING_PATH));
            for section in VOCABULARY_SECTIONS {
                assert!(resource_paths.contains(vocabulary_path(section).as_str()));
            }
        }
    }
}
