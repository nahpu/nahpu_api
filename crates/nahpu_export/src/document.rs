use crate::models::ExportData;

/// Represents the engine that converts `ExportData` records into formatted documents.
pub struct DocumentExport {
    /// The parsed database records containing narratives, sites, events, and specimens.
    pub data: ExportData,
}

impl DocumentExport {
    /// Constructs a new `DocumentExport` by deserializing the provided JSON data into `ExportData`.
    pub fn new(json_data: &str) -> Result<Self, serde_json::Error> {
        let data: ExportData = serde_json::from_str(json_data)?;
        Ok(Self { data })
    }

    /// Generates a standard Markdown (`.md`) representation of the database records.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();

        if let Some(narratives) = &self.data.narrative {
            out.push_str("# Narratives\n\n");
            for record in narratives {
                if let Some(date) = &record.date {
                    out.push_str(&format!("**Date**: {}\n\n", escape_markdown(date)));
                }
                if let Some(site) = &record.site_id {
                    out.push_str(&format!("**Site ID**: {}\n\n", escape_markdown(&site.to_string())));
                }
                if let Some(nar) = &record.narrative {
                    out.push_str(&format!("{}\n\n", escape_markdown(nar)));
                }
                out.push_str("---\n\n");
            }
        }

        if let Some(sites) = &self.data.sites {
            out.push_str("# Sites\n\n");
            for record in sites {
                if let Some(loc) = &record.locality {
                    out.push_str(&format!("**Locality**: {}\n\n", escape_markdown(loc)));
                }
                if let Some(country) = &record.country {
                    out.push_str(&format!("**Country**: {}\n\n", escape_markdown(country)));
                }
                out.push_str("---\n\n");
            }
        }

        if let Some(events) = &self.data.events {
            out.push_str("# Events\n\n");
            for record in events {
                if let Some(start) = &record.start_date {
                    out.push_str(&format!("**Start Date**: {}\n\n", escape_markdown(start)));
                }
                if let Some(end) = &record.end_date {
                    out.push_str(&format!("**End Date**: {}\n\n", escape_markdown(end)));
                }
                out.push_str("---\n\n");
            }
        }

        if let Some(specimens) = &self.data.specimens {
            out.push_str("# Specimens\n\n");
            for record in specimens {
                if let Some(fnum) = &record.field_number {
                    out.push_str(&format!("**Field Number**: {}\n\n", escape_markdown(&fnum.to_string())));
                }
                if let Some(cond) = &record.condition {
                    out.push_str(&format!("**Condition**: {}\n\n", escape_markdown(cond)));
                }
                out.push_str("---\n\n");
            }
        }

        out
    }

    /// Generates a Typst (`.typ`) source code representation of the database records.
    pub fn to_typst(&self) -> String {
        let mut out = String::new();

        if let Some(narratives) = &self.data.narrative {
            out.push_str("= Narratives\n\n");
            for record in narratives {
                if let Some(date) = &record.date {
                    out.push_str(&format!("*Date*: {}\n\n", escape_typst(date)));
                }
                if let Some(site) = &record.site_id {
                    out.push_str(&format!("*Site ID*: {}\n\n", escape_typst(&site.to_string())));
                }
                if let Some(nar) = &record.narrative {
                    out.push_str(&format!("{}\n\n", escape_typst(nar)));
                }
                out.push_str("#line(length: 100%)\n\n");
            }
        }

        if let Some(sites) = &self.data.sites {
            out.push_str("= Sites\n\n");
            for record in sites {
                if let Some(loc) = &record.locality {
                    out.push_str(&format!("*Locality*: {}\n\n", escape_typst(loc)));
                }
                if let Some(country) = &record.country {
                    out.push_str(&format!("*Country*: {}\n\n", escape_typst(country)));
                }
                out.push_str("#line(length: 100%)\n\n");
            }
        }

        if let Some(events) = &self.data.events {
            out.push_str("= Events\n\n");
            for record in events {
                if let Some(start) = &record.start_date {
                    out.push_str(&format!("*Start Date*: {}\n\n", escape_typst(start)));
                }
                if let Some(end) = &record.end_date {
                    out.push_str(&format!("*End Date*: {}\n\n", escape_typst(end)));
                }
                out.push_str("#line(length: 100%)\n\n");
            }
        }

        if let Some(specimens) = &self.data.specimens {
            out.push_str("= Specimens\n\n");
            for record in specimens {
                if let Some(fnum) = &record.field_number {
                    out.push_str(&format!("*Field Number*: {}\n\n", escape_typst(&fnum.to_string())));
                }
                if let Some(cond) = &record.condition {
                    out.push_str(&format!("*Condition*: {}\n\n", escape_typst(cond)));
                }
                out.push_str("#line(length: 100%)\n\n");
            }
        }

        out
    }

    /// Helper for returning PDF bytes from Typst. We pass font_bytes as an argument
    /// which can be loaded in Dart (since Flutter can easily read assets as bytes).
    pub fn to_pdf(&self, font_bytes: Vec<Vec<u8>>) -> Result<Vec<u8>, String> {
        let typst_source = self.to_typst();
        crate::typst_compiler::compile_to_pdf(&typst_source, font_bytes).map_err(|e| e.to_string())
    }
}

fn escape_markdown(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 10);
    for c in text.chars() {
        match c {
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.' | '!' | '|' | '<' | '>' => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }
    escaped
}

fn escape_typst(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 10);
    for c in text.chars() {
        match c {
            '#' | '$' | '*' | '_' | '<' | '>' | '@' | '\\' | '[' | ']' | '~' | '`' => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }
    escaped
}

