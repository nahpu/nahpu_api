# nahpu_export

Document rendering for the NAHPU specimen cataloging app.

`DocumentRenderer` converts typed NAHPU database records to Markdown or Typst.
`TypstCompiler` compiles generated or caller-provided Typst into PDF bytes with
caller-provided fonts. Markdown-to-Typst conversion supports the rich-text
constructs used by NAHPU templates.

```rust
use nahpu_export::{DocumentRenderer, ExportData, TypstCompiler};

let renderer = DocumentRenderer::new(ExportData {
    narrative: None,
    sites: None,
    events: None,
    specimens: None,
});
let typst = renderer.render_typst();
let pdf = TypstCompiler::new(vec![]).compile(&typst)?;
# Ok::<(), nahpu_export::ExportError>(())
```

The crate returns typed `ExportError` values for JSON parsing, Typst diagnostics,
and PDF encoding failures.
