use chrono::{DateTime, Datelike, Local};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

use crate::ExportError;

/// Represents a Typst environment needed to compile a single document.
/// It contains a virtual file system to allow document previews and a font book.
struct TypstWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: Source,
}

impl TypstWorld {
    /// Constructs a world from Typst source and parsed fonts.
    fn new(source_text: String, fonts: Vec<Font>) -> Self {
        let mut book = FontBook::new();
        for font in &fonts {
            book.push(font.info().clone());
        }

        let file_id = FileId::new(None, VirtualPath::new("main.typ"));
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            source: Source::new(file_id, source_text),
        }
    }
}

impl World for TypstWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id.vpath() == &VirtualPath::new("main.typ") {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(
                id.vpath().as_rootless_path().to_path_buf(),
            ))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().as_rootless_path();
        // Generated sources use absolute image paths, so first read the path directly.
        if let Ok(bytes) = std::fs::read(path) {
            return Ok(bytes.into());
        }

        // On Unix and macOS, restore the leading slash removed by Typst.
        let with_slash = std::path::Path::new("/").join(path);
        if let Ok(bytes) = std::fs::read(&with_slash) {
            return Ok(bytes.into());
        }

        // On Windows, restore drive-letter paths such as C/Users or C:/Users.
        #[cfg(windows)]
        {
            let path_str = path.to_string_lossy();
            if path_str.len() >= 2 && path_str.as_bytes()[1] == b'/' {
                let drive = &path_str[0..1];
                let rest = &path_str[2..];
                let win_path = format!("{}:\\{}", drive, rest.replace('/', "\\"));
                if let Ok(bytes) = std::fs::read(&win_path) {
                    return Ok(bytes.into());
                }
            }
        }

        Err(FileError::NotFound(path.to_path_buf()))
    }

    fn font(&self, id: usize) -> Option<Font> {
        self.fonts.get(id).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        let now: DateTime<Local> = Local::now();
        Datetime::from_ymd(now.year(), now.month() as u8, now.day() as u8)
    }
}

/// Reusable compiler for Typst source and caller-provided fonts.
pub struct TypstCompiler {
    fonts: Vec<Font>,
}

impl TypstCompiler {
    /// Creates a compiler from raw font files.
    pub fn new(font_bytes: Vec<Vec<u8>>) -> Self {
        let fonts = font_bytes
            .into_iter()
            .flat_map(|bytes| Font::iter(Bytes::from(bytes)))
            .collect();
        Self { fonts }
    }

    /// Compiles Typst source into PDF bytes.
    pub fn compile(&self, source: &str) -> Result<Vec<u8>, ExportError> {
        let world = TypstWorld::new(source.to_owned(), self.fonts.clone());

        let document = typst::compile(&world)
            .output
            .map_err(|diagnostics| ExportError::TypstCompilation(format!("{diagnostics:?}")))?;

        typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .map_err(|error| ExportError::PdfGeneration(format!("{error:?}")))
    }
}
