use chrono::{DateTime, Datelike, Local};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

/// Represents a Typst environment needed to compile a single document.
/// It contains a virtual file system to allow document previews and a font book.
pub struct SimpleWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: Source,
}

impl SimpleWorld {
    /// Constructs a `SimpleWorld` using the provided raw Typst source code and a list of font bytes.
    pub fn new(source_text: String, fonts_bytes: Vec<Vec<u8>>) -> Self {
        let mut fonts = Vec::new();
        for bytes in fonts_bytes {
            let bytes: Bytes = bytes.into();
            fonts.extend(Font::iter(bytes));
        }

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

impl World for SimpleWorld {
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
        // Since we are generating the Typst source dynamically and we pass absolute paths for images,
        // we can attempt to read from the path directly.
        if let Ok(bytes) = std::fs::read(path) {
            return Ok(bytes.into());
        }

        // On Unix/macOS, try prepending `/` to make it absolute if it was originally an absolute path
        let with_slash = std::path::Path::new("/").join(path);
        if let Ok(bytes) = std::fs::read(&with_slash) {
            return Ok(bytes.into());
        }

        // On Windows, handle cases where the path starts with a drive letter (e.g. C/Users/... or C:/Users/...)
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

/// Compiles the provided Typst source code into a PDF binary.
pub fn compile_to_pdf(source: &str, fonts_bytes: Vec<Vec<u8>>) -> Result<Vec<u8>, String> {
    let world = SimpleWorld::new(source.to_string(), fonts_bytes);

    let document = typst::compile(&world)
        .output
        .map_err(|err| format!("Compilation error: {:?}", err))?;

    let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|err| format!("PDF generation error: {:?}", err))?;

    Ok(pdf_bytes)
}
