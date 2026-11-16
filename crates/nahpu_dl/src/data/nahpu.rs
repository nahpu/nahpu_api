//! Specimen images from an extracted NAHPU data package.

use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::data::{LabeledDataset, Sample, SampleInput, Target};
use crate::{DlError, Result};

/// Project file at the root of a NAHPU data package.
pub const PROJECT_FILE_NAME: &str = "nahpu-project.json";

/// Taxonomic rank used as the class label.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxonRank {
    /// Genus and specific epithet, such as "Rattus rattus".
    #[default]
    Species,
    /// Genus.
    Genus,
    /// Family.
    Family,
    /// Order.
    Order,
    /// Class.
    Class,
}

impl std::str::FromStr for TaxonRank {
    type Err = DlError;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "species" => Ok(Self::Species),
            "genus" => Ok(Self::Genus),
            "family" => Ok(Self::Family),
            "order" => Ok(Self::Order),
            "class" => Ok(Self::Class),
            other => Err(DlError::Config(format!(
                "unknown taxon rank `{other}`; expected species, genus, family, order, or class"
            ))),
        }
    }
}

/// Reads specimen photos and their identifications from an extracted NAHPU data package.
///
/// The package is the directory the NAHPU app writes when it exports a project: a
/// `nahpu-project.json` with the project records and a `media/` directory with the files.
/// Images are linked to specimens through `specimenMedia`, and labeled by the specimen's
/// taxon at [`NahpuPackage::rank`].
///
/// Sample ids are `<catalog>:<media id>`, where the catalog part is the specimen's museum
/// ID, or its UUID when there is none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NahpuPackage {
    /// Package directory, holding `nahpu-project.json`.
    pub dir: PathBuf,
    /// Rank used for labels.
    pub rank: TaxonRank,
    /// Only use media of this category. All categories are used when unset.
    pub media_category: Option<String>,
}

impl NahpuPackage {
    /// Creates a reader labeling images by species.
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            rank: TaxonRank::Species,
            media_category: None,
        }
    }

    /// Whether `dir` looks like an extracted NAHPU data package.
    pub fn is_package(dir: &Path) -> bool {
        dir.join(PROJECT_FILE_NAME).is_file()
    }

    /// Sets the label rank.
    pub fn with_rank(mut self, rank: TaxonRank) -> Self {
        self.rank = rank;
        self
    }

    /// Restricts images to a media category.
    pub fn with_media_category(mut self, category: impl Into<String>) -> Self {
        self.media_category = Some(category.into());
        self
    }

    /// Reads specimen images labeled by taxon. Images whose specimen has no taxon at the
    /// chosen rank are skipped and counted.
    ///
    /// # Errors
    ///
    /// Returns an error when the project file cannot be parsed, a media path is unsafe, or
    /// a media file is missing.
    pub fn read_labeled(&self) -> Result<LabeledDataset> {
        let project = self.read_project()?;
        let mut dataset = LabeledDataset::default();
        for image in project.specimen_images(self)? {
            match image.label {
                Some(label) => {
                    dataset.samples.push(image.sample);
                    dataset.targets.push(Target::Class(label));
                }
                None => dataset.skipped_rows += 1,
            }
        }
        if dataset.is_empty() {
            return Err(DlError::Data(format!(
                "no identified specimen images in {}",
                self.dir.display()
            )));
        }
        Ok(dataset)
    }

    /// Reads every specimen image, identified or not.
    ///
    /// # Errors
    ///
    /// Returns an error when the project file cannot be parsed, a media path is unsafe, or
    /// a media file is missing.
    pub fn read_samples(&self) -> Result<Vec<Sample>> {
        let project = self.read_project()?;
        Ok(project
            .specimen_images(self)?
            .into_iter()
            .map(|image| image.sample)
            .collect())
    }

    fn read_project(&self) -> Result<ProjectFile> {
        let path = self.dir.join(PROJECT_FILE_NAME);
        let text = fs::read_to_string(&path).map_err(|e| DlError::io(&path, e))?;
        serde_json::from_str(&text)
            .map_err(|e| DlError::Data(format!("cannot parse {}: {e}", path.display())))
    }
}

/// The parts of `nahpu-project.json` this reader needs.
///
/// Column names follow the NAHPU Drift schema exactly. They are spelled out here because
/// names such as `speciesID` do not survive serde's camelCase conversion.
#[derive(Debug, Default, Deserialize)]
struct ProjectFile {
    #[serde(default)]
    records: Records,
    #[serde(default)]
    media: Vec<MediaFileEntry>,
}

#[derive(Debug, Default, Deserialize)]
struct Records {
    #[serde(default)]
    specimen: Vec<SpecimenRow>,
    #[serde(default)]
    media: Vec<MediaRow>,
    #[serde(default, rename = "specimenMedia")]
    specimen_media: Vec<SpecimenMediaRow>,
    #[serde(default)]
    taxonomy: Vec<TaxonRow>,
}

#[derive(Debug, Deserialize)]
struct SpecimenRow {
    uuid: String,
    #[serde(default, rename = "speciesID")]
    species_id: Option<i64>,
    #[serde(default, rename = "museumID")]
    museum_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MediaRow {
    #[serde(rename = "primaryId")]
    primary_id: i64,
    #[serde(default)]
    category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpecimenMediaRow {
    #[serde(rename = "specimenUuid")]
    specimen_uuid: String,
    #[serde(default, rename = "mediaId")]
    media_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TaxonRow {
    id: i64,
    #[serde(default, rename = "taxonClass")]
    taxon_class: Option<String>,
    #[serde(default, rename = "taxonOrder")]
    taxon_order: Option<String>,
    #[serde(default, rename = "taxonFamily")]
    taxon_family: Option<String>,
    #[serde(default)]
    genus: Option<String>,
    #[serde(default, rename = "specificEpithet")]
    specific_epithet: Option<String>,
}

impl TaxonRow {
    fn label(&self, rank: TaxonRank) -> Option<String> {
        let label = match rank {
            TaxonRank::Species => match (&self.genus, &self.specific_epithet) {
                (Some(genus), Some(epithet)) => {
                    Some(format!("{} {}", genus.trim(), epithet.trim()))
                }
                _ => None,
            },
            TaxonRank::Genus => self.genus.clone(),
            TaxonRank::Family => self.taxon_family.clone(),
            TaxonRank::Order => self.taxon_order.clone(),
            TaxonRank::Class => self.taxon_class.clone(),
        };
        label
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
    }
}

/// Entry of the package media manifest: `sourceId` is `media:<primaryId>`.
#[derive(Debug, Deserialize)]
struct MediaFileEntry {
    #[serde(rename = "sourceId")]
    source_id: String,
    #[serde(rename = "archivePath")]
    archive_path: String,
}

struct SpecimenImage {
    sample: Sample,
    label: Option<String>,
}

impl ProjectFile {
    fn specimen_images(&self, package: &NahpuPackage) -> Result<Vec<SpecimenImage>> {
        let specimens: HashMap<&str, &SpecimenRow> = self
            .records
            .specimen
            .iter()
            .map(|s| (s.uuid.as_str(), s))
            .collect();
        let taxa: HashMap<i64, &TaxonRow> =
            self.records.taxonomy.iter().map(|t| (t.id, t)).collect();
        let media: HashMap<i64, &MediaRow> = self
            .records
            .media
            .iter()
            .map(|m| (m.primary_id, m))
            .collect();
        let files: HashMap<&str, &str> = self
            .media
            .iter()
            .map(|entry| (entry.source_id.as_str(), entry.archive_path.as_str()))
            .collect();
        let mut images = Vec::new();
        for link in &self.records.specimen_media {
            let Some(media_id) = link.media_id else {
                continue;
            };
            let Some(row) = media.get(&media_id) else {
                continue;
            };
            if !package.category_matches(row) {
                continue;
            }
            // The app leaves out media whose file was missing at export.
            let Some(archive_path) = files.get(format!("media:{media_id}").as_str()) else {
                continue;
            };
            let Some(specimen) = specimens.get(link.specimen_uuid.as_str()) else {
                continue;
            };
            let path = package.resolve(archive_path)?;
            let catalog = specimen.museum_id.as_deref().unwrap_or(&specimen.uuid);
            let label = specimen
                .species_id
                .and_then(|id| taxa.get(&id))
                .and_then(|taxon| taxon.label(package.rank));
            images.push(SpecimenImage {
                sample: Sample::new(
                    format!("{catalog}:{media_id}"),
                    SampleInput::ImagePath(path),
                ),
                label,
            });
        }
        images.sort_by(|a, b| a.sample.id.cmp(&b.sample.id));
        Ok(images)
    }
}

impl NahpuPackage {
    fn category_matches(&self, media: &MediaRow) -> bool {
        match &self.media_category {
            Some(category) => media.category.as_deref() == Some(category.as_str()),
            None => true,
        }
    }

    /// Joins a package-relative path, refusing paths that leave the package.
    fn resolve(&self, archive_path: &str) -> Result<PathBuf> {
        let relative = Path::new(archive_path);
        if !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(DlError::Data(format!("unsafe media path `{archive_path}`")));
        }
        let path = self.dir.join(relative);
        if !path.is_file() {
            return Err(DlError::Data(format!(
                "media file {} is missing",
                path.display()
            )));
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A package with three specimen photos: two identified, one not, plus a site photo.
    fn write_package(dir: &Path) {
        let project = serde_json::json!({
            "nahpu_project": "project",
            "version": 4,
            "records": {
                "specimen": [
                    {"uuid": "s1", "speciesID": 1, "museumID": "MVZ 1"},
                    {"uuid": "s2", "speciesID": 2},
                    {"uuid": "s3", "speciesID": null}
                ],
                "taxonomy": [
                    {"id": 1, "genus": "Rattus", "specificEpithet": "rattus", "taxonFamily": "Muridae"},
                    {"id": 2, "genus": "Mus", "specificEpithet": "musculus", "taxonFamily": "Muridae"}
                ],
                "media": [
                    {"primaryId": 10, "category": "specimen", "fileName": "a.jpg"},
                    {"primaryId": 11, "category": "specimen", "fileName": "b.jpg"},
                    {"primaryId": 12, "category": "specimen", "fileName": "c.jpg"},
                    {"primaryId": 13, "category": "site", "fileName": "d.jpg"}
                ],
                "specimenMedia": [
                    {"specimenUuid": "s1", "mediaId": 10},
                    {"specimenUuid": "s2", "mediaId": 11},
                    {"specimenUuid": "s3", "mediaId": 12},
                    {"specimenUuid": "s1", "mediaId": 13}
                ]
            },
            "media": [
                {"sourceId": "media:10", "kind": "specimen", "archivePath": "media/10-a.jpg", "originalFileName": "a.jpg"},
                {"sourceId": "media:11", "kind": "specimen", "archivePath": "media/11-b.jpg", "originalFileName": "b.jpg"},
                {"sourceId": "media:12", "kind": "specimen", "archivePath": "media/12-c.jpg", "originalFileName": "c.jpg"},
                {"sourceId": "media:13", "kind": "site", "archivePath": "media/13-d.jpg", "originalFileName": "d.jpg"}
            ]
        });
        fs::create_dir_all(dir.join("media")).unwrap();
        fs::write(dir.join(PROJECT_FILE_NAME), project.to_string()).unwrap();
        for name in ["10-a.jpg", "11-b.jpg", "12-c.jpg", "13-d.jpg"] {
            image::RgbImage::new(4, 4)
                .save(dir.join("media").join(name))
                .unwrap();
        }
    }

    #[test]
    fn test_read_labeled_by_species() {
        let dir = tempfile::tempdir().unwrap();
        write_package(dir.path());
        let dataset = NahpuPackage::new(dir.path())
            .with_media_category("specimen")
            .read_labeled()
            .unwrap();
        let ids: Vec<&str> = dataset.samples.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["MVZ 1:10", "s2:11"]);
        assert_eq!(
            dataset.class_labels(),
            vec!["Mus musculus", "Rattus rattus"]
        );
        assert_eq!(dataset.skipped_rows, 1);
    }

    #[test]
    fn test_read_labeled_by_family_and_all_categories() {
        let dir = tempfile::tempdir().unwrap();
        write_package(dir.path());
        let dataset = NahpuPackage::new(dir.path())
            .with_rank(TaxonRank::Family)
            .read_labeled()
            .unwrap();
        assert_eq!(dataset.len(), 3);
        assert_eq!(dataset.class_labels(), vec!["Muridae"]);
        assert_eq!(
            NahpuPackage::new(dir.path()).read_samples().unwrap().len(),
            4
        );
    }

    #[test]
    fn test_rejects_paths_outside_the_package() {
        let dir = tempfile::tempdir().unwrap();
        write_package(dir.path());
        let package = NahpuPackage::new(dir.path());
        assert!(package.resolve("../escape.jpg").is_err());
        assert!(package.resolve("/etc/passwd").is_err());
        assert!(package.resolve("media/10-a.jpg").is_ok());
    }

    #[test]
    fn test_parse_taxon_rank() {
        assert_eq!("Genus".parse::<TaxonRank>().unwrap(), TaxonRank::Genus);
        assert!("tribe".parse::<TaxonRank>().is_err());
    }
}
