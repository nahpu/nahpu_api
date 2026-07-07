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
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(md_text, options);
    let mut out = String::with_capacity(md_text.len() * 2);
    let mut list_stack = Vec::new();
    let mut in_code_block = false;

    fn push_newline(s: &mut String) {
        if !s.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
    }

    fn push_double_newline(s: &mut String) {
        if !s.is_empty() {
            if s.ends_with("\n\n") {
                // Do nothing
            } else if s.ends_with('\n') {
                s.push('\n');
            } else {
                s.push_str("\n\n");
            }
        }
    }

    for event in parser {
        match event {
            Event::Text(text) => {
                if in_code_block {
                    out.push_str(&text);
                } else {
                    out.push_str(&escape_typst(&text));
                }
            }
            Event::Code(code) => out.push_str(&format!("`{}`", code)),
            Event::Html(html) | Event::InlineHtml(html) => out.push_str(&escape_typst(&html)),
            Event::Start(Tag::Heading { level, .. }) => {
                let hashes = "=".repeat(level as usize);
                out.push_str(&format!("{} ", hashes));
            }
            Event::End(TagEnd::Heading(_level)) => push_double_newline(&mut out),
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => push_double_newline(&mut out),
            Event::Start(Tag::BlockQuote(_)) => out.push_str("#quote["),
            Event::End(TagEnd::BlockQuote(_)) => {
                out.push(']');
                push_double_newline(&mut out);
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                out.push_str("```\n");
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                out.push_str("```\n\n");
            }
            Event::Start(Tag::List(None)) => {
                if !list_stack.is_empty() {
                    push_newline(&mut out);
                }
                list_stack.push(false);
            }
            Event::Start(Tag::List(Some(_))) => {
                if !list_stack.is_empty() {
                    push_newline(&mut out);
                }
                list_stack.push(true);
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                push_newline(&mut out);
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                out.push_str(&indent);
                if let Some(&is_ordered) = list_stack.last() {
                    if is_ordered {
                        out.push_str("+ ");
                    } else {
                        out.push_str("- ");
                    }
                } else {
                    out.push_str("- ");
                }
            }
            Event::End(TagEnd::Item) => push_newline(&mut out),
            Event::Start(Tag::Table(alignments)) => {
                let mut align_strs = Vec::new();
                for align in &alignments {
                    let s = match align {
                        pulldown_cmark::Alignment::Left => "left",
                        pulldown_cmark::Alignment::Center => "center",
                        pulldown_cmark::Alignment::Right => "right",
                        pulldown_cmark::Alignment::None => "left",
                    };
                    align_strs.push(s);
                }
                out.push_str(&format!("#table(\n  columns: {},\n  align: ({}),\n", alignments.len(), align_strs.join(", ")));
            }
            Event::End(TagEnd::Table) => {
                out.push_str(")\n\n");
            }
            Event::Start(Tag::TableCell) => {
                out.push_str("  [");
            }
            Event::End(TagEnd::TableCell) => {
                out.push_str("],\n");
            }
            Event::Start(Tag::TableHead) | Event::End(TagEnd::TableHead) | Event::Start(Tag::TableRow) | Event::End(TagEnd::TableRow) => {}
            Event::Start(Tag::Strong) => out.push('*'),
            Event::End(TagEnd::Strong) => out.push('*'),
            Event::Start(Tag::Emphasis) => out.push('_'),
            Event::End(TagEnd::Emphasis) => out.push('_'),
            Event::Start(Tag::Strikethrough) => out.push_str("#strike["),
            Event::End(TagEnd::Strikethrough) => out.push(']'),
            Event::Start(Tag::Link { dest_url, .. }) => {
                let clean_url = dest_url.replace('"', "\\\"");
                out.push_str(&format!("#link(\"{}\")[", clean_url));
            }
            Event::End(TagEnd::Link) => out.push(']'),
            Event::Start(Tag::Image { dest_url, .. }) => {
                let clean_url = dest_url.replace('"', "\\\"");
                out.push_str(&format!("#image(\"{}\")", clean_url));
            }
            Event::End(TagEnd::Image) => {}
            Event::HardBreak => out.push_str("\\ \n"),
            Event::SoftBreak => out.push('\n'),
            Event::Rule => {
                push_newline(&mut out);
                out.push_str("#line(length: 100%)\n\n");
            }
            Event::FootnoteReference(name) => out.push_str(&format!("#footnote[{}]", name)),
            Event::TaskListMarker(checked) => {
                if checked {
                    out.push_str("[x] ");
                } else {
                    out.push_str("[ ] ");
                }
            }
            _ => {}
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
    fn test_markdown_to_typst_nested_lists() {
        let md = "Some **bold** and *italic* text.\n\n- Item 1\n  - Nested Item 1a\n  - Nested Item 1b\n- Item 2\n\n1. Numbered 1\n   1. Nested Numbered 1a\n2. Numbered 2";
        let typst = markdown_to_typst(md);
        println!("Generated Typst nested lists:\n{}", typst);
        assert!(typst.contains("- Item 1\n  - Nested Item 1a\n  - Nested Item 1b\n- Item 2"));
        assert!(typst.contains("+ Numbered 1\n  + Nested Numbered 1a\n+ Numbered 2"));
    }

    #[test]
    fn test_markdown_to_typst_code_blocks() {
        let md = "Inline `code_with_underscore` and block:\n\n```rust\nfn my_function_name() {\n    let x = 42;\n}\n```";
        let typst = markdown_to_typst(md);
        println!("Generated Typst code blocks:\n{}", typst);
        assert!(typst.contains("`code_with_underscore`"));
        assert!(typst.contains("my_function_name"));
        assert!(!typst.contains("code\\_with\\_underscore"));
        assert!(!typst.contains("my\\_function\\_name"));
    }

    #[test]
    fn test_markdown_to_typst_links_images() {
        let md = "[link_text](https://example.com/some_url_with_underscores) and ![alt_text](img_with_underscores.png)";
        let typst = markdown_to_typst(md);
        println!("Generated Typst links/images:\n{}", typst);
        assert!(typst.contains("#link(\"https://example.com/some_url_with_underscores\")"));
        assert!(typst.contains("#image(\"img_with_underscores.png\")"));
    }

    #[test]
    fn test_markdown_to_typst_table() {
        let md = "| Header 1 | Header 2 |\n|---|---|\n| Cell 1 | Cell 2 |";
        let typst = markdown_to_typst(md);
        println!("Generated Typst table:\n{}", typst);
        assert!(typst.contains("#table("));
        assert!(typst.contains("columns: 2"));
        assert!(typst.contains("[Header 1]"));
        assert!(typst.contains("[Cell 1]"));
    }

    #[test]
    fn test_pdf_compilation() {
        let md = "Some **bold** and *italic* text.\n\n- Item 1\n- Item 2\n\n1. Numbered 1\n2. Numbered 2";
        let typst = markdown_to_typst(md);
        let pdf_res = crate::typst_compiler::compile_to_pdf(&typst, vec![]);
        assert!(pdf_res.is_ok(), "Failed to compile: {:?}", pdf_res.err());
    }
}

