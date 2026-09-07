//! The registry of standard terms NAHPU is allowed to write into a Darwin Core bundle.
//!
//! A bundle CSV column exists only when it is registered here. Each entry names the CSV
//! header, the standard term that header stands for, and the namespace the term is
//! published under. Headers were resolved against the ratified Darwin Core list of terms
//! (2026-05-26) and the Darwin Core Data Package 1.0 table schemas; the profile defines no
//! namespace of its own, so every Data Package column versions a Darwin Core, Dublin Core,
//! or Audubon Core term.
//!
//! Values with no registered term belong in a NAHPU Data Package, which is lossless.

/// The controlled namespace a registered term is published under.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TermNamespace {
    DarwinCore,
    DublinCore,
    AudubonCore,
}

impl TermNamespace {
    /// The base IRI terms in this namespace resolve against.
    pub const fn base(self) -> &'static str {
        match self {
            Self::DarwinCore => "http://rs.tdwg.org/dwc/terms/",
            Self::DublinCore => "http://purl.org/dc/terms/",
            Self::AudubonCore => "http://rs.tdwg.org/ac/terms/",
        }
    }

    /// The prefix NAHPU writes for this namespace in prefixed headers.
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::DarwinCore => "dwc",
            Self::DublinCore => "dcterms",
            Self::AudubonCore => "ac",
        }
    }
}

/// The bundle writer a table belongs to.
///
/// The two writers publish different shapes: the Archive is one flat occurrence core with
/// extensions, while the Data Package is a normalized relational model. A header is only
/// ever legal for one table of one profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleProfile {
    Archive,
    DataPackage,
}

/// One registered column of one bundle CSV table.
#[derive(Clone, Copy, Debug)]
pub struct BundleTerm {
    /// The CSV header exactly as written.
    pub header: &'static str,
    /// The term the header stands for. It differs from `header` for prefixed headers
    /// (`dcterms:title` stands for `title`) and for the Data Package surrogate keys
    /// (`occurrence_pk` versions `occurrenceID`).
    pub term: &'static str,
    pub namespace: TermNamespace,
    /// Whether the Frictionless descriptor types this column as an integer.
    pub integer: bool,
}

/// The registered columns of one bundle CSV table.
#[derive(Clone, Copy, Debug)]
pub struct TableTerms {
    pub table: &'static str,
    pub profile: BundleProfile,
    pub terms: &'static [BundleTerm],
}

const fn dwc(header: &'static str) -> BundleTerm {
    BundleTerm {
        header,
        term: header,
        namespace: TermNamespace::DarwinCore,
        integer: false,
    }
}

const fn dwc_as(header: &'static str, term: &'static str) -> BundleTerm {
    BundleTerm {
        header,
        term,
        namespace: TermNamespace::DarwinCore,
        integer: false,
    }
}

const fn dcterms_as(header: &'static str, term: &'static str) -> BundleTerm {
    BundleTerm {
        header,
        term,
        namespace: TermNamespace::DublinCore,
        integer: false,
    }
}

const fn ac_as(header: &'static str, term: &'static str) -> BundleTerm {
    BundleTerm {
        header,
        term,
        namespace: TermNamespace::AudubonCore,
        integer: false,
    }
}

const fn counted(header: &'static str) -> BundleTerm {
    BundleTerm {
        header,
        term: header,
        namespace: TermNamespace::DarwinCore,
        integer: true,
    }
}

const AR_MATERIAL_TERMS: &[BundleTerm] = &[
    dwc("catalogNumber"),
    dwc("eventID"),
    dwc("materialEntityID"),
    dwc("materialEntityRemarks"),
    dwc("materialEntityType"),
    dwc("materialSampleID"),
    dwc("objectQuantity"),
    dwc("objectQuantityType"),
    dwc("occurrenceID"),
    dwc("otherCatalogNumbers"),
    dwc("preparations"),
];

const AR_MEASUREMENT_OR_FACT_TERMS: &[BundleTerm] = &[
    dwc("measurementID"),
    dwc("measurementType"),
    dwc("measurementUnit"),
    dwc("measurementValue"),
    dwc("occurrenceID"),
];

const AR_MULTIMEDIA_TERMS: &[BundleTerm] = &[
    ac_as("ac:accessURI", "accessURI"),
    dcterms_as("dcterms:created", "created"),
    dcterms_as("dcterms:creator", "creator"),
    dcterms_as("dcterms:description", "description"),
    dcterms_as("dcterms:identifier", "identifier"),
    dcterms_as("dcterms:title", "title"),
    dcterms_as("dcterms:type", "type"),
    dwc("occurrenceID"),
];

const AR_OCCURRENCE_TERMS: &[BundleTerm] = &[
    dwc("associatedOccurrences"),
    dwc("associatedTaxa"),
    dwc("basisOfRecord"),
    dwc("caste"),
    dwc("catalogNumber"),
    dwc("class"),
    dwc("coordinateUncertaintyInMeters"),
    dwc("country"),
    dwc("county"),
    dwc("decimalLatitude"),
    dwc("decimalLongitude"),
    dwc("eventDate"),
    dwc("eventID"),
    dwc("eventTime"),
    dwc("family"),
    dwc("genus"),
    dwc("geodeticDatum"),
    dwc("georeferenceRemarks"),
    dwc("habitat"),
    dwc("identificationType"),
    dwc("identificationVerificationStatus"),
    dwc("identifiedBy"),
    dwc("identifiedByID"),
    dwc("individualCount"),
    dwc("infraspecificEpithet"),
    dwc("islandGroup"),
    dwc("kingdom"),
    dwc("lifeStage"),
    dwc("locality"),
    dwc("locationRemarks"),
    dwc("maximumElevationInMeters"),
    dwc("minimumElevationInMeters"),
    dwc("municipality"),
    dwc("occurrenceID"),
    dwc("occurrenceRemarks"),
    dwc("occurrenceStatus"),
    dwc("order"),
    dwc("otherCatalogNumbers"),
    dwc("phylum"),
    dwc("preparations"),
    dwc("recordNumber"),
    dwc("recordedBy"),
    dwc("recordedByID"),
    dwc("reproductiveCondition"),
    dwc("samplingEffort"),
    dwc("samplingProtocol"),
    dwc("scientificName"),
    dwc("scientificNameAuthorship"),
    dwc("sex"),
    dwc("specificEpithet"),
    dwc("stateProvince"),
    dwc("taxonID"),
    dwc("taxonRank"),
    dwc("taxonRemarks"),
    dwc("verbatimCoordinateSystem"),
    dwc("verbatimCoordinates"),
    dwc("verbatimLatitude"),
    dwc("verbatimLongitude"),
    dwc("vernacularName"),
];

const AR_RESOURCE_RELATIONSHIP_TERMS: &[BundleTerm] = &[
    dwc("relatedResourceID"),
    dwc("relationshipOfResource"),
    dwc("relationshipRemarks"),
    dwc("resourceID"),
    dwc("resourceRelationshipID"),
];

const DA_AGENT_TERMS: &[BundleTerm] = &[
    dwc("agentID"),
    dwc("agentType"),
    dwc_as("agent_pk", "agentID"),
    dcterms_as("preferredAgentName", "title"),
];

const DA_EVENT_TERMS: &[BundleTerm] = &[
    dwc("coordinateUncertaintyInMeters"),
    dwc("country"),
    dwc("county"),
    dwc("decimalLatitude"),
    dwc("decimalLongitude"),
    dwc("eventCategory"),
    dwc("eventDate"),
    dwc("eventID"),
    dwc("eventRemarks"),
    dwc("eventTime"),
    dwc_as("event_pk", "eventID"),
    dwc("geodeticDatum"),
    dwc("georeferenceRemarks"),
    dwc("habitat"),
    dwc("islandGroup"),
    dwc("locality"),
    dwc("locationID"),
    dwc("locationRemarks"),
    dwc("maximumElevationInMeters"),
    dwc("minimumElevationInMeters"),
    dwc("municipality"),
    dwc("stateProvince"),
    dwc("verbatimCoordinateSystem"),
    dwc("verbatimCoordinates"),
    dwc("verbatimLatitude"),
    dwc("verbatimLongitude"),
];

const DA_EVENT_AGENT_ROLE_TERMS: &[BundleTerm] = &[
    dwc_as("agentRole", "relationshipOfResource"),
    counted("agentRoleOrder"),
    dwc_as("agent_fk", "agentID"),
    dwc_as("event_fk", "eventID"),
];

const DA_EVENT_ASSERTION_TERMS: &[BundleTerm] = &[
    dwc("assertionID"),
    dwc("assertionType"),
    dwc("assertionUnit"),
    dwc("assertionValue"),
    dwc_as("event_fk", "eventID"),
];

const DA_IDENTIFICATION_TERMS: &[BundleTerm] = &[
    dwc("class"),
    dwc("family"),
    dwc("genus"),
    dwc("identificationID"),
    dwc("identificationType"),
    dwc("identificationVerificationStatus"),
    dwc_as("identification_pk", "identificationID"),
    dwc("identifiedBy"),
    dwc("identifiedByID"),
    dwc("infraspecificEpithet"),
    dwc("kingdom"),
    dwc_as("occurrence_fk", "occurrenceID"),
    dwc("order"),
    dwc("phylum"),
    dwc("scientificName"),
    dwc("scientificNameAuthorship"),
    dwc("specificEpithet"),
    dwc("taxonID"),
    dwc("taxonRank"),
    dwc("taxonRemarks"),
    dwc("vernacularName"),
];

const DA_MATERIAL_TERMS: &[BundleTerm] = &[
    dwc("catalogNumber"),
    dwc_as("collectionEvent_fk", "eventID"),
    dwc_as("evidenceForOccurrence_fk", "occurrenceID"),
    dwc("materialEntityID"),
    dwc("materialEntityRemarks"),
    dwc("materialEntityType"),
    dwc_as("materialEntity_pk", "materialEntityID"),
    dwc("objectQuantity"),
    dwc("objectQuantityType"),
    dwc("otherCatalogNumbers"),
    dwc("preparations"),
];

const DA_MATERIAL_AGENT_ROLE_TERMS: &[BundleTerm] = &[
    dwc_as("agentRole", "relationshipOfResource"),
    counted("agentRoleOrder"),
    dwc_as("agent_fk", "agentID"),
    dwc_as("materialEntity_fk", "materialEntityID"),
];

const DA_MATERIAL_ASSERTION_TERMS: &[BundleTerm] = &[
    dwc("assertionID"),
    dwc("assertionType"),
    dwc("assertionUnit"),
    dwc("assertionValue"),
    dwc_as("materialEntity_fk", "materialEntityID"),
];

const DA_MEDIA_TERMS: &[BundleTerm] = &[
    ac_as("accessURI", "accessURI"),
    dcterms_as("mediaID", "identifier"),
    dcterms_as("mediaType", "type"),
    dcterms_as("media_pk", "identifier"),
    dcterms_as("title", "title"),
];

const DA_MEDIA_AGENT_ROLE_TERMS: &[BundleTerm] = &[
    dwc_as("agentRole", "relationshipOfResource"),
    counted("agentRoleOrder"),
    dwc_as("agent_fk", "agentID"),
    dcterms_as("media_fk", "identifier"),
];

const DA_OCCURRENCE_TERMS: &[BundleTerm] = &[
    dwc("caste"),
    dwc_as("event_fk", "eventID"),
    dwc("identificationVerificationStatus"),
    dwc("identifiedBy"),
    dwc("identifiedByID"),
    dwc("lifeStage"),
    dwc("occurrenceID"),
    dwc("occurrenceRemarks"),
    dwc("occurrenceStatus"),
    dwc_as("occurrence_pk", "occurrenceID"),
    dwc("organismQuantity"),
    dwc("organismQuantityType"),
    dwc("recordNumber"),
    dwc("reproductiveCondition"),
    dwc("scientificName"),
    dwc("scientificNameAuthorship"),
    dwc("sex"),
    dwc("taxonID"),
    dwc("taxonRank"),
    dwc("vernacularName"),
];

const DA_OCCURRENCE_AGENT_ROLE_TERMS: &[BundleTerm] = &[
    dwc_as("agentRole", "relationshipOfResource"),
    counted("agentRoleOrder"),
    dwc_as("agent_fk", "agentID"),
    dwc_as("occurrence_fk", "occurrenceID"),
];

const DA_OCCURRENCE_ASSERTION_TERMS: &[BundleTerm] = &[
    dwc("assertionID"),
    dwc("assertionType"),
    dwc("assertionUnit"),
    dwc("assertionValue"),
    dwc_as("occurrence_fk", "occurrenceID"),
];

const DA_OCCURRENCE_MEDIA_TERMS: &[BundleTerm] = &[
    dcterms_as("media_fk", "identifier"),
    dwc_as("occurrence_fk", "occurrenceID"),
];

const DA_ORGANISM_INTERACTION_TERMS: &[BundleTerm] = &[
    dwc_as("event_fk", "eventID"),
    dwc("organismInteractionDescription"),
    dwc("organismInteractionID"),
    dwc("organismInteractionType"),
    dwc_as("organismInteraction_pk", "organismInteractionID"),
    dwc_as("relatedOccurrence_fk", "occurrenceID"),
    dwc_as("relatedOrganismPart", "organismPart"),
    dwc_as("subjectOccurrence_fk", "occurrenceID"),
];

const DA_ORGANISM_INTERACTION_ASSERTION_TERMS: &[BundleTerm] = &[
    dwc("assertionID"),
    dwc("assertionType"),
    dwc("assertionUnit"),
    dwc("assertionValue"),
    dwc_as("organismInteraction_fk", "organismInteractionID"),
];

/// Standard terms NAHPU writes in flat tabular exports but never as a bundle column.
///
/// The bundle tables above are the authority for what a bundle may contain; this list
/// keeps the shared vocabulary complete so preset headers can be validated too.
pub const ADDITIONAL_REGISTERED_TERMS: &[BundleTerm] = &[
    dcterms_as("date", "date"),
    dcterms_as("modified", "modified"),
    dcterms_as("subject", "subject"),
    dwc("agentRemarks"),
    dwc("datasetID"),
    dwc("earliestAgeOrLowestStage"),
    dwc("formation"),
    dwc("higherClassification"),
    dwc("latestAgeOrHighestStage"),
    dwc("projectID"),
    dwc("projectTitle"),
    dwc("verbatimLocality"),
];

/// Every bundle CSV table NAHPU writes, with the columns it may carry.
pub const BUNDLE_TABLES: &[TableTerms] = &[
    TableTerms {
        table: "material",
        profile: BundleProfile::Archive,
        terms: AR_MATERIAL_TERMS,
    },
    TableTerms {
        table: "measurement_or_fact",
        profile: BundleProfile::Archive,
        terms: AR_MEASUREMENT_OR_FACT_TERMS,
    },
    TableTerms {
        table: "multimedia",
        profile: BundleProfile::Archive,
        terms: AR_MULTIMEDIA_TERMS,
    },
    TableTerms {
        table: "occurrence",
        profile: BundleProfile::Archive,
        terms: AR_OCCURRENCE_TERMS,
    },
    TableTerms {
        table: "resource_relationship",
        profile: BundleProfile::Archive,
        terms: AR_RESOURCE_RELATIONSHIP_TERMS,
    },
    TableTerms {
        table: "agent",
        profile: BundleProfile::DataPackage,
        terms: DA_AGENT_TERMS,
    },
    TableTerms {
        table: "event",
        profile: BundleProfile::DataPackage,
        terms: DA_EVENT_TERMS,
    },
    TableTerms {
        table: "event-agent-role",
        profile: BundleProfile::DataPackage,
        terms: DA_EVENT_AGENT_ROLE_TERMS,
    },
    TableTerms {
        table: "event-assertion",
        profile: BundleProfile::DataPackage,
        terms: DA_EVENT_ASSERTION_TERMS,
    },
    TableTerms {
        table: "identification",
        profile: BundleProfile::DataPackage,
        terms: DA_IDENTIFICATION_TERMS,
    },
    TableTerms {
        table: "material",
        profile: BundleProfile::DataPackage,
        terms: DA_MATERIAL_TERMS,
    },
    TableTerms {
        table: "material-agent-role",
        profile: BundleProfile::DataPackage,
        terms: DA_MATERIAL_AGENT_ROLE_TERMS,
    },
    TableTerms {
        table: "material-assertion",
        profile: BundleProfile::DataPackage,
        terms: DA_MATERIAL_ASSERTION_TERMS,
    },
    TableTerms {
        table: "media",
        profile: BundleProfile::DataPackage,
        terms: DA_MEDIA_TERMS,
    },
    TableTerms {
        table: "media-agent-role",
        profile: BundleProfile::DataPackage,
        terms: DA_MEDIA_AGENT_ROLE_TERMS,
    },
    TableTerms {
        table: "occurrence",
        profile: BundleProfile::DataPackage,
        terms: DA_OCCURRENCE_TERMS,
    },
    TableTerms {
        table: "occurrence-agent-role",
        profile: BundleProfile::DataPackage,
        terms: DA_OCCURRENCE_AGENT_ROLE_TERMS,
    },
    TableTerms {
        table: "occurrence-assertion",
        profile: BundleProfile::DataPackage,
        terms: DA_OCCURRENCE_ASSERTION_TERMS,
    },
    TableTerms {
        table: "occurrence-media",
        profile: BundleProfile::DataPackage,
        terms: DA_OCCURRENCE_MEDIA_TERMS,
    },
    TableTerms {
        table: "organism-interaction",
        profile: BundleProfile::DataPackage,
        terms: DA_ORGANISM_INTERACTION_TERMS,
    },
    TableTerms {
        table: "organism-interaction-assertion",
        profile: BundleProfile::DataPackage,
        terms: DA_ORGANISM_INTERACTION_ASSERTION_TERMS,
    },
];

/// Resolves bundle CSV headers to the standard terms NAHPU is allowed to write.
pub struct TermRegistry;

impl TermRegistry {
    /// Resolves one candidate column of one table for one bundle writer.
    ///
    /// `None` means the header has no registered standard term in that position and must
    /// not be written.
    pub fn column(
        table: &str,
        header: &str,
        profile: BundleProfile,
    ) -> Option<&'static BundleTerm> {
        Self::table_terms(table, profile)?
            .iter()
            .find(|term| term.header == header)
    }

    /// The absolute IRI advertised for a registered column.
    pub fn term_uri(term: &BundleTerm) -> String {
        format!("{}{}", term.namespace.base(), term.term)
    }

    /// Every registered table, for descriptor generation and cross-checking tests.
    pub fn tables() -> &'static [TableTerms] {
        BUNDLE_TABLES
    }

    /// Whether a prefixed tabular header such as `dwc:sex` names a registered term.
    pub fn is_registered_prefixed(header: &str) -> bool {
        let Some((prefix, name)) = header.split_once(':') else {
            return false;
        };
        let matches = |term: &BundleTerm| term.term == name && term.namespace.prefix() == prefix;
        BUNDLE_TABLES
            .iter()
            .any(|table| table.terms.iter().any(matches))
            || ADDITIONAL_REGISTERED_TERMS.iter().any(matches)
    }

    fn table_terms(table: &str, profile: BundleProfile) -> Option<&'static [BundleTerm]> {
        BUNDLE_TABLES
            .iter()
            .find(|entry| entry.table == table && entry.profile == profile)
            .map(|entry| entry.terms)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ADDITIONAL_REGISTERED_TERMS, BUNDLE_TABLES, BundleProfile, TermNamespace, TermRegistry,
    };

    #[test]
    fn bundle_tables_are_sorted_and_unique() {
        for table in BUNDLE_TABLES {
            let headers = table
                .terms
                .iter()
                .map(|term| term.header)
                .collect::<Vec<_>>();
            let mut sorted = headers.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(
                headers, sorted,
                "{} is unsorted or repeats a header",
                table.table
            );
        }
    }

    #[test]
    fn every_registered_term_resolves_to_a_standard_namespace() {
        let bases = [
            TermNamespace::DarwinCore.base(),
            TermNamespace::DublinCore.base(),
            TermNamespace::AudubonCore.base(),
        ];
        let terms = BUNDLE_TABLES
            .iter()
            .flat_map(|table| table.terms.iter())
            .chain(ADDITIONAL_REGISTERED_TERMS.iter());
        for term in terms {
            let uri = TermRegistry::term_uri(term);
            assert!(
                bases.iter().any(|base| uri.starts_with(base)),
                "{uri} is outside the registered namespaces"
            );
            assert!(
                !uri.contains("dwc-dp"),
                "{uri} uses the Data Package namespace, which defines no terms"
            );
        }
    }

    #[test]
    fn surrogate_keys_version_an_identifier_term() {
        for table in BUNDLE_TABLES {
            for term in table.terms {
                if !term.header.ends_with("_pk") && !term.header.ends_with("_fk") {
                    continue;
                }
                assert_eq!(
                    table.profile,
                    BundleProfile::DataPackage,
                    "{} is an Archive table and cannot carry {}",
                    table.table,
                    term.header
                );
                assert_ne!(
                    term.header, term.term,
                    "{} must version an identifier term",
                    term.header
                );
            }
        }
    }

    #[test]
    fn archive_and_data_package_resolve_columns_separately() {
        assert!(
            TermRegistry::column("occurrence", "occurrence_pk", BundleProfile::Archive).is_none()
        );
        assert!(
            TermRegistry::column("occurrence", "occurrence_pk", BundleProfile::DataPackage)
                .is_some()
        );
        assert!(TermRegistry::column("occurrence", "genus", BundleProfile::Archive).is_some());
        assert!(TermRegistry::column("occurrence", "genus", BundleProfile::DataPackage).is_none());
        assert!(TermRegistry::column("occurrence", "notATerm", BundleProfile::Archive).is_none());
    }

    #[test]
    fn prefixed_headers_resolve_against_the_shared_vocabulary() {
        assert!(TermRegistry::is_registered_prefixed("dwc:sex"));
        assert!(TermRegistry::is_registered_prefixed("dcterms:title"));
        assert!(TermRegistry::is_registered_prefixed("ac:accessURI"));
        assert!(TermRegistry::is_registered_prefixed("dwc:projectTitle"));
        assert!(!TermRegistry::is_registered_prefixed(
            "dwc:preferredAgentName"
        ));
        assert!(!TermRegistry::is_registered_prefixed("sex"));
    }
}
