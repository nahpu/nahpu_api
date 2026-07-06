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
                    out.push_str(&format!(
                        "**Site ID**: {}\n\n",
                        escape_markdown(&site.to_string())
                    ));
                }
                if let Some(nar) = &record.narrative {
                    out.push_str(&format!("{}\n\n", nar));
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
                    out.push_str(&format!(
                        "**Field Number**: {}\n\n",
                        escape_markdown(&fnum.to_string())
                    ));
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
                    out.push_str(&format!(
                        "*Site ID*: {}\n\n",
                        escape_typst(&site.to_string())
                    ));
                }
                if let Some(nar) = &record.narrative {
                    out.push_str(&format!("{}\n\n", markdown_to_typst(nar)));
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
                    out.push_str(&format!(
                        "*Field Number*: {}\n\n",
                        escape_typst(&fnum.to_string())
                    ));
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

/// Converts Markdown text into Typst markup.
pub fn markdown_to_typst(md_text: &str) -> String {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};

    let parser = Parser::new(md_text);
    let mut out = String::with_capacity(md_text.len() * 2);

    for event in parser {
        match event {
            Event::Text(text) => out.push_str(&escape_typst(&text)),
            Event::Code(code) => out.push_str(&format!("`{}`", escape_typst(&code))),
            Event::Html(html) | Event::InlineHtml(html) => out.push_str(&escape_typst(&html)),
            Event::Start(Tag::Heading { level, .. }) => {
                let hashes = "=".repeat(level as usize);
                out.push_str(&format!("{} ", hashes));
            }
            Event::End(TagEnd::Heading(_level)) => out.push_str("\n\n"),
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => out.push_str("\n\n"),
            Event::Start(Tag::BlockQuote(_)) => out.push_str("#quote["),
            Event::End(TagEnd::BlockQuote(_)) => out.push_str("]\n\n"),
            Event::Start(Tag::CodeBlock(_)) => out.push_str("```\n"),
            Event::End(TagEnd::CodeBlock) => out.push_str("```\n\n"),
            Event::Start(Tag::List(None)) => {} // Unordered list
            Event::Start(Tag::List(Some(_))) => {} // Ordered list
            Event::End(TagEnd::List(_)) => out.push('\n'),
            Event::Start(Tag::Item) => out.push_str("- "),
            Event::End(TagEnd::Item) => out.push('\n'),
            Event::Start(Tag::Strong) => out.push('*'),
            Event::End(TagEnd::Strong) => out.push('*'),
            Event::Start(Tag::Emphasis) => out.push('_'),
            Event::End(TagEnd::Emphasis) => out.push('_'),
            Event::Start(Tag::Strikethrough) => out.push_str("#strike["),
            Event::End(TagEnd::Strikethrough) => out.push(']'),
            Event::Start(Tag::Link { dest_url, .. }) => {
                out.push_str(&format!("#link(\"{}\")[", dest_url));
            }
            Event::End(TagEnd::Link) => out.push(']'),
            Event::Start(Tag::Image { dest_url, .. }) => {
                out.push_str(&format!("#image(\"{}\")", dest_url));
            }
            Event::End(TagEnd::Image) => {}
            Event::HardBreak => out.push_str("\\ \n"),
            Event::SoftBreak => out.push('\n'),
            Event::Rule => out.push_str("#line(length: 100%)\n\n"),
            Event::FootnoteReference(name) => out.push_str(&format!("#footnote[{}]", name)),
            Event::TaskListMarker(checked) => {
                if checked {
                    out.push_str("[x] ");
                } else {
                    out.push_str("[ ] ");
                }
            }
            _ => {} // Ignore other events
        }
    }

    out.trim().to_string()
}

fn escape_markdown(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 10);
    for c in text.chars() {
        match c {
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' | '<' | '>' => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_typst() {
        let md = "Some **bold** and *italic* text.\n\n- Item 1\n- Item 2\n\n1. Numbered 1\n2. Numbered 2";
        let typst = markdown_to_typst(md);
        println!("Generated Typst:\n{}", typst);
    }

    #[test]
    fn test_pdf_compilation() {
        let md = "Some **bold** and *italic* text.\n\n- Item 1\n- Item 2\n\n1. Numbered 1\n2. Numbered 2";
        let typst = markdown_to_typst(md);
        let pdf_res = crate::typst_compiler::compile_to_pdf(&typst, vec![]);
        assert!(pdf_res.is_ok(), "Failed to compile: {:?}", pdf_res.err());
    }
}

