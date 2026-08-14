# cooklang-format

Render [Cooklang](https://cooklang.org) recipes into text formats.

Extracted from [CookCLI](https://github.com/cooklang/cookcli), where these
formatters back `cook recipe -f <format>`. They are published separately so
other projects can render recipes without depending on the CLI's internals.

| module | output |
|---|---|
| `markdown` | Markdown, with the metadata as YAML front matter |
| `human` | terminal text, optionally ANSI-styled |
| `cooklang_source` | Cooklang source (round-trips) |
| `latex`, `typst` | typeset documents, paper-size and margin aware |
| `schema` | schema.org/Recipe JSON-LD |
| `number`, `quantity` | shared primitives: number rendering, and the deterministic ordering every other module renders grouped quantities through |

JSON and YAML of the recipe itself need no formatter here: `cooklang::Recipe`
is `Serialize`, so `serde_json` or `serde_yaml` handle those directly.

## Usage

```rust
use cooklang_format::cooklang::{Converter, CooklangParser, Extensions};
use cooklang_format::markdown_to_string;

let parser = CooklangParser::new(Extensions::empty(), Converter::default());
let source = "---\ntitle: Tea\n---\n\nBoil @water{2%cups} in a #pot.\n";
let (recipe, _) = parser.parse(source).into_result().unwrap();

let markdown = markdown_to_string(&recipe, "Tea", 1.0, parser.converter()).unwrap();
assert!(markdown.contains("water"));
```

The `print_*` functions are the primitives: each writes into a
`std::io::Write`, so a caller already holding a file or socket does not pay for
a second copy of the document. `human_to_string` and `markdown_to_string` are
`String`-returning wrappers over the two most commonly wanted ones.

The `cooklang` crate these functions take their types from is re-exported at
this crate's root, so a consumer can name `Recipe` and `Converter` without
adding — and having to keep in step — a `cooklang` dependency of its own.

## Colour

`Style::Plain` is the default and emits no escape codes. Colour is passed
explicitly rather than through `yansi`'s global switch, so a library consumer
cannot have escape sequences appear in a file it is writing.

## License

MIT. See [LICENSE](LICENSE).

The `human`, `markdown` and `cooklang_source` modules include a substantial
portion of code from [cooklang-chef](https://github.com/Zheoni/cooklang-chef),
Copyright (c) 2023 Francisco J. Sanchez, also under MIT license. Each of those
files carries the original notice in full at the top.
