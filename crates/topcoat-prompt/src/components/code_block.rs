use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, Child, StaticClass, Unescaped, View, class, component, view},
};

use super::copy_button::copy_button;

/// Classes for the bordered block. The header and body share its rounded corners.
const CODE_BLOCK: StaticClass = class!(
    "flex w-full min-w-0 flex-col overflow-hidden rounded-lg border border-border bg-card",
    "text-card-foreground",
);

/// The token colors, as classes for the element around highlighted code.
///
/// Each color mixes a hue with the card's text color, so one rule is darker on a light
/// card and lighter on a dark one: no theme tokens to add, and it follows the theme.
/// Comments use the muted text color. Put these classes on your own `<code>` element
/// to color [`highlight`] output outside a [`code_block`].
pub const TOKEN_COLORS: StaticClass = class!(
    "[&_.tk-comment]:[color:var(--muted-foreground)]",
    "[&_.tk-meta]:[color:var(--muted-foreground)]",
    "[&_.tk-keyword]:[color:color-mix(in_oklab,oklch(0.6_0.2_15)_62%,var(--card-foreground))]",
    "[&_.tk-string]:[color:color-mix(in_oklab,oklch(0.58_0.14_245)_62%,var(--card-foreground))]",
    "[&_.tk-number]:[color:color-mix(in_oklab,oklch(0.6_0.17_255)_70%,var(--card-foreground))]",
    "[&_.tk-property]:[color:color-mix(in_oklab,oklch(0.6_0.17_255)_70%,var(--card-foreground))]",
    "[&_.tk-function]:[color:color-mix(in_oklab,oklch(0.6_0.19_300)_66%,var(--card-foreground))]",
    "[&_.tk-type]:[color:color-mix(in_oklab,oklch(0.64_0.15_55)_70%,var(--card-foreground))]",
    "[&_.tk-variable]:[color:color-mix(in_oklab,oklch(0.64_0.15_55)_70%,var(--card-foreground))]",
    "[&_.tk-tag]:[color:color-mix(in_oklab,oklch(0.6_0.15_150)_66%,var(--card-foreground))]",
    "[&_.tk-inserted]:[color:color-mix(in_oklab,oklch(0.6_0.15_150)_66%,var(--card-foreground))]",
    "[&_.tk-deleted]:[color:color-mix(in_oklab,var(--destructive)_75%,var(--card-foreground))]",
);

/// A block of code with a header naming its language and a copy button.
///
/// The code keeps its whitespace and scrolls horizontally instead of wrapping.
/// `language` labels the block, is added to the `<code>` element as a `language-*`
/// class, and picks the highlighting: see [`highlight`] for the languages and their
/// aliases. Code in another language, or without one, is shown as plain text. Set
/// `highlight` to `false` to show plain text anyway.
///
/// Highlighting runs on the server each time the block renders, so a block that grows
/// while a reply streams is highlighted as far as it goes. It takes well under a
/// millisecond for a few hundred lines.
///
/// To use another highlighter, pass its markup as child content: it replaces the text
/// inside `<code>`, while `code` is still what the copy button copies. The markup must
/// be the same text as `code`, wrapped in elements.
///
/// `attrs` are forwarded to the outer `<div>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     code_block(language: Some("rust"), code: "fn main() {\n    println!(\"hi\");\n}")
/// }
/// ```
#[component]
pub async fn code_block(
    cx: &Cx,
    /// The language of the code, such as `rust`.
    #[default]
    language: Option<&str>,
    /// The code.
    code: &str,
    /// Whether to highlight `code` when its language is supported.
    #[default(true)]
    highlight: bool,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// Highlighted markup to show instead of the plain `code`.
    #[into]
    #[default]
    child: Option<Child<'_>>,
) -> Result<impl View> {
    let code_class = language.map(|language| format!("language-{language}"));
    let highlighted = match (&child, language) {
        (None, Some(language)) if highlight => self::highlight(language, code),
        _ => None,
    };

    Ok(view! {
        <div
            data-slot="code-block"
            class=(class!(CODE_BLOCK, attrs.remove("class")))
            (attrs)
        >
            code_block_header(language: language, code_block_copy(text: code))
            <pre class="overflow-x-auto p-4 font-mono text-sm leading-relaxed">
                <code class=(class!(TOKEN_COLORS, code_class))>
                    match (child, highlighted) {
                        (Some(markup), _) => (markup),
                        (
                            // The highlighter escapes the code: see `highlight`.
                            None,
                            Some(html),
                        ) => (Unescaped::new_unchecked(html)),
                        (None, None) => (code),
                    }
                </code>
            </pre>
        </div>
    })
}

/// The header of a [`code_block`]: the language label on the left and actions, such
/// as a [`code_block_copy`] button, on the right.
///
/// Without a language, the label reads "Code". `attrs` are forwarded to the `<div>`.
/// Extra classes are added to its classes.
#[component]
pub async fn code_block_header(
    cx: &Cx,
    /// The language shown as the label.
    #[default]
    language: Option<&str>,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The actions on the right.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "flex h-10 items-center justify-between gap-2 border-b border-border",
                "bg-foreground/5 pr-0.5 pl-4 text-xs text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            <span class="truncate font-mono lowercase">
                (language.unwrap_or("code"))
            </span>
            <div class="flex items-center">(child)</div>
        </div>
    })
}

/// The copy button of a [`code_block`], labelled "Copy code".
///
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
#[component]
pub async fn code_block_copy(
    cx: &Cx,
    /// The code written to the clipboard.
    #[into]
    text: String,
    /// Extra attributes for the `<button>` element.
    #[default]
    attrs: Attributes,
) -> Result<impl View> {
    Ok(view! { copy_button(text: text, label: "Copy code", attrs: attrs) })
}

/// Highlights `code` as `language`, returning HTML, or `None` when the language is not
/// supported.
///
/// The languages, by name and alias:
///
/// - Rust (`rust`, `rs`), Go (`go`, `golang`), C (`c`, `h`), C++ (`cpp`, `c++`, `cc`,
///   `hpp`), Java (`java`) and C# (`csharp`, `cs`, `c#`)
/// - Python (`python`, `py`)
/// - JavaScript (`javascript`, `js`, `mjs`, `cjs`, `jsx`) and TypeScript (`typescript`,
///   `ts`, `mts`, `cts`, `tsx`), with JSX in `jsx` and `tsx`
/// - JSON (`json`, `jsonc`, `json5`), YAML (`yaml`, `yml`) and TOML (`toml`)
/// - Bash (`bash`, `sh`, `shell`, `zsh`, `console`, `shellsession`)
/// - HTML (`html`, `htm`, `xml`, `svg`, `vue`, `svelte`), with scripts and styles in
///   it, and CSS (`css`, `scss`, `less`)
/// - SQL (`sql`, `postgres`, `postgresql`, `mysql`, `sqlite`)
/// - Diffs (`diff`, `patch`)
///
/// Names are matched without regard to case. The output is the code with its special
/// characters escaped and its tokens wrapped in `<span class="tk-*">` elements, which
/// [`TOKEN_COLORS`] colors, so it is safe to render unescaped. The highlighter is a
/// lexer, not a parser: it never fails, and code that stops part way, such as a block
/// that is still streaming, is highlighted as far as it goes, with an unclosed string or
/// comment running to the end.
pub fn highlight(language: &str, code: &str) -> Option<String> {
    let language = language.trim().to_ascii_lowercase();
    let lexer = match language.as_str() {
        "rust" | "rs" => Lexer::Code(&RUST),
        "python" | "py" | "python3" | "py3" => Lexer::Code(&PYTHON),
        "javascript" | "js" | "mjs" | "cjs" | "node" => Lexer::Code(&JAVASCRIPT),
        "jsx" => Lexer::Code(&JSX),
        "typescript" | "ts" | "mts" | "cts" => Lexer::Code(&TYPESCRIPT),
        "tsx" => Lexer::Code(&TSX),
        "go" | "golang" => Lexer::Code(&GO),
        "c" | "h" => Lexer::Code(&C),
        "cpp" | "c++" | "cc" | "cxx" | "hpp" => Lexer::Code(&CPP),
        "java" => Lexer::Code(&JAVA),
        "csharp" | "cs" | "c#" => Lexer::Code(&CSHARP),
        "bash" | "sh" | "shell" | "zsh" | "console" | "shellsession" => Lexer::Code(&BASH),
        "sql" | "postgres" | "postgresql" | "mysql" | "sqlite" | "plsql" => Lexer::Code(&SQL),
        "json" | "jsonc" | "json5" => Lexer::Json,
        "yaml" | "yml" => Lexer::Yaml,
        "toml" => Lexer::Toml,
        "html" | "htm" | "xml" | "svg" | "xhtml" | "vue" | "svelte" => Lexer::Html,
        "css" => Lexer::Css {
            line_comments: false,
        },
        "scss" | "less" => Lexer::Css {
            line_comments: true,
        },
        "diff" | "patch" => Lexer::Diff,
        _ => return None,
    };
    let mut out = Out(String::with_capacity(code.len() * 2), None);
    lexer.run(code, &mut out, 0);
    Some(out.0)
}

/// How deeply languages may nest, such as JSX inside a template string inside JSX.
/// Deeper code is left plain, so hostile input cannot exhaust the stack.
const MAX_DEPTH: usize = 12;

/// The kind of a token, which is its class.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Comment,
    Meta,
    Keyword,
    String,
    Number,
    Property,
    Function,
    Type,
    Variable,
    Tag,
    Inserted,
    Deleted,
}

impl Kind {
    fn class(self) -> &'static str {
        match self {
            Self::Comment => "tk-comment",
            Self::Meta => "tk-meta",
            Self::Keyword => "tk-keyword",
            Self::String => "tk-string",
            Self::Number => "tk-number",
            Self::Property => "tk-property",
            Self::Function => "tk-function",
            Self::Type => "tk-type",
            Self::Variable => "tk-variable",
            Self::Tag => "tk-tag",
            Self::Inserted => "tk-inserted",
            Self::Deleted => "tk-deleted",
        }
    }
}

/// The highlighted HTML being written, and the kind of the span it ends with.
struct Out(String, Option<Kind>);

impl Out {
    /// Appends `text`, escaped, and wrapped in a span when it has a kind. Text of the
    /// same kind as the span before it joins that span.
    fn push(&mut self, kind: Option<Kind>, text: &str) {
        if text.is_empty() {
            return;
        }
        if kind.is_some() && kind == self.1 {
            self.0.truncate(self.0.len() - "</span>".len());
        } else if let Some(kind) = kind {
            self.0.push_str("<span class=\"");
            self.0.push_str(kind.class());
            self.0.push_str("\">");
        }
        self.1 = kind;
        for c in text.chars() {
            match c {
                '&' => self.0.push_str("&amp;"),
                '<' => self.0.push_str("&lt;"),
                '>' => self.0.push_str("&gt;"),
                '"' => self.0.push_str("&quot;"),
                '\'' => self.0.push_str("&#39;"),
                c => self.0.push(c),
            }
        }
        if kind.is_some() {
            self.0.push_str("</span>");
        }
    }
}

enum Lexer {
    Code(&'static Syntax),
    Json,
    Yaml,
    Toml,
    Html,
    Css { line_comments: bool },
    Diff,
}

impl Lexer {
    fn run(&self, src: &str, out: &mut Out, depth: usize) {
        if depth > MAX_DEPTH {
            out.push(None, src);
            return;
        }
        match self {
            Self::Code(syntax) => code(src, syntax, out, depth),
            Self::Json => json(src, out),
            Self::Yaml => yaml(src, out),
            Self::Toml => toml(src, out),
            Self::Html => html(src, out, depth),
            Self::Css { line_comments } => css(src, *line_comments, out),
            Self::Diff => diff(src, out),
        }
    }
}

/// The rules that set the code-like languages apart.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flavor {
    Rust,
    Python,
    /// JavaScript and TypeScript, with JSX when `jsx` is set.
    Js {
        jsx: bool,
    },
    Go,
    /// C, C++, Java and C#.
    C,
    Bash,
    Sql,
}

/// A code-like language: its words and its comments.
struct Syntax {
    flavor: Flavor,
    keywords: &'static [&'static str],
    /// Literals such as `true` and `null`.
    constants: &'static [&'static str],
    /// Built-in type names.
    types: &'static [&'static str],
    line_comment: &'static str,
    block_comments: bool,
}

const RUST: Syntax = Syntax {
    flavor: Flavor::Rust,
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
        "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "type",
        "unsafe", "use", "where", "while", "yield",
    ],
    constants: &["true", "false"],
    types: &[
        "bool", "char", "str", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32",
        "i64", "i128", "isize", "f32", "f64",
    ],
    line_comment: "//",
    block_comments: true,
};

const PYTHON: Syntax = Syntax {
    flavor: Flavor::Python,
    keywords: &[
        "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
        "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is",
        "lambda", "match", "case", "nonlocal", "not", "or", "pass", "raise", "return", "try",
        "while", "with", "yield", "self", "cls",
    ],
    constants: &["True", "False", "None"],
    types: &[
        "int", "float", "str", "bool", "bytes", "list", "dict", "set", "tuple", "object",
    ],
    line_comment: "#",
    block_comments: false,
};

const JS_KEYWORDS: &[&str] = &[
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "finally",
    "for",
    "from",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
    "get",
    "set",
];

const TS_KEYWORDS: &[&str] = &[
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "finally",
    "for",
    "from",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
    "get",
    "set",
    "abstract",
    "as",
    "declare",
    "enum",
    "implements",
    "interface",
    "keyof",
    "namespace",
    "private",
    "protected",
    "public",
    "readonly",
    "satisfies",
    "type",
    "infer",
    "is",
];

const JS_CONSTANTS: &[&str] = &["true", "false", "null", "undefined", "NaN", "Infinity"];

const TS_TYPES: &[&str] = &[
    "string", "number", "boolean", "bigint", "symbol", "any", "unknown", "never", "object",
];

const JAVASCRIPT: Syntax = Syntax {
    flavor: Flavor::Js { jsx: false },
    keywords: JS_KEYWORDS,
    constants: JS_CONSTANTS,
    types: &[],
    line_comment: "//",
    block_comments: true,
};

const JSX: Syntax = Syntax {
    flavor: Flavor::Js { jsx: true },
    ..JAVASCRIPT
};

const TYPESCRIPT: Syntax = Syntax {
    flavor: Flavor::Js { jsx: false },
    keywords: TS_KEYWORDS,
    constants: JS_CONSTANTS,
    types: TS_TYPES,
    line_comment: "//",
    block_comments: true,
};

const TSX: Syntax = Syntax {
    flavor: Flavor::Js { jsx: true },
    ..TYPESCRIPT
};

const GO: Syntax = Syntax {
    flavor: Flavor::Go,
    keywords: &[
        "break",
        "case",
        "chan",
        "const",
        "continue",
        "default",
        "defer",
        "else",
        "fallthrough",
        "for",
        "func",
        "go",
        "goto",
        "if",
        "import",
        "interface",
        "map",
        "package",
        "range",
        "return",
        "select",
        "struct",
        "switch",
        "type",
        "var",
    ],
    constants: &["true", "false", "nil", "iota"],
    types: &[
        "bool",
        "byte",
        "complex64",
        "complex128",
        "error",
        "float32",
        "float64",
        "int",
        "int8",
        "int16",
        "int32",
        "int64",
        "rune",
        "string",
        "uint",
        "uint8",
        "uint16",
        "uint32",
        "uint64",
        "uintptr",
        "any",
    ],
    line_comment: "//",
    block_comments: true,
};

const C_TYPES: &[&str] = &[
    "void", "char", "short", "int", "long", "float", "double", "signed", "unsigned", "bool",
    "size_t", "int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t",
    "uint64_t", "auto",
];

const C: Syntax = Syntax {
    flavor: Flavor::C,
    keywords: &[
        "break", "case", "const", "continue", "default", "do", "else", "enum", "extern", "for",
        "goto", "if", "inline", "register", "restrict", "return", "sizeof", "static", "struct",
        "switch", "typedef", "union", "volatile", "while",
    ],
    constants: &["true", "false", "NULL"],
    types: C_TYPES,
    line_comment: "//",
    block_comments: true,
};

const CPP: Syntax = Syntax {
    keywords: &[
        "break",
        "case",
        "catch",
        "class",
        "const",
        "constexpr",
        "continue",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "explicit",
        "export",
        "extern",
        "for",
        "friend",
        "goto",
        "if",
        "inline",
        "mutable",
        "namespace",
        "new",
        "noexcept",
        "operator",
        "override",
        "private",
        "protected",
        "public",
        "return",
        "sizeof",
        "static",
        "struct",
        "switch",
        "template",
        "this",
        "throw",
        "try",
        "typedef",
        "typename",
        "union",
        "using",
        "virtual",
        "volatile",
        "while",
        "co_await",
        "co_return",
    ],
    constants: &["true", "false", "nullptr", "NULL"],
    ..C
};

const JAVA: Syntax = Syntax {
    keywords: &[
        "abstract",
        "assert",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "else",
        "enum",
        "extends",
        "final",
        "finally",
        "for",
        "if",
        "implements",
        "import",
        "instanceof",
        "interface",
        "native",
        "new",
        "package",
        "private",
        "protected",
        "public",
        "record",
        "return",
        "static",
        "super",
        "switch",
        "synchronized",
        "this",
        "throw",
        "throws",
        "try",
        "var",
        "volatile",
        "while",
        "yield",
    ],
    constants: &["true", "false", "null"],
    types: &[
        "boolean", "byte", "char", "double", "float", "int", "long", "short", "void", "String",
    ],
    ..C
};

const CSHARP: Syntax = Syntax {
    keywords: &[
        "abstract",
        "as",
        "async",
        "await",
        "base",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "delegate",
        "do",
        "else",
        "enum",
        "event",
        "explicit",
        "extern",
        "finally",
        "fixed",
        "for",
        "foreach",
        "get",
        "if",
        "implicit",
        "in",
        "init",
        "interface",
        "internal",
        "is",
        "lock",
        "namespace",
        "new",
        "operator",
        "out",
        "override",
        "params",
        "private",
        "protected",
        "public",
        "readonly",
        "record",
        "ref",
        "return",
        "sealed",
        "set",
        "sizeof",
        "static",
        "struct",
        "switch",
        "this",
        "throw",
        "try",
        "typeof",
        "using",
        "var",
        "virtual",
        "when",
        "where",
        "while",
        "yield",
    ],
    constants: &["true", "false", "null"],
    types: &[
        "bool", "byte", "char", "decimal", "double", "dynamic", "float", "int", "long", "object",
        "sbyte", "short", "string", "uint", "ulong", "ushort", "void",
    ],
    ..C
};

const BASH: Syntax = Syntax {
    flavor: Flavor::Bash,
    keywords: &[
        "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac",
        "in", "function", "return", "export", "local", "readonly", "declare", "unset", "select",
        "time",
    ],
    constants: &["true", "false"],
    types: &[],
    line_comment: "#",
    block_comments: false,
};

const SQL: Syntax = Syntax {
    flavor: Flavor::Sql,
    keywords: &[
        "add",
        "all",
        "alter",
        "and",
        "any",
        "as",
        "asc",
        "begin",
        "between",
        "by",
        "cascade",
        "case",
        "check",
        "column",
        "commit",
        "constraint",
        "create",
        "cross",
        "database",
        "default",
        "delete",
        "desc",
        "distinct",
        "drop",
        "else",
        "end",
        "exists",
        "foreign",
        "from",
        "full",
        "group",
        "having",
        "if",
        "in",
        "index",
        "inner",
        "insert",
        "into",
        "is",
        "join",
        "key",
        "left",
        "like",
        "limit",
        "not",
        "offset",
        "on",
        "or",
        "order",
        "outer",
        "over",
        "partition",
        "primary",
        "references",
        "returning",
        "right",
        "rollback",
        "select",
        "set",
        "table",
        "then",
        "transaction",
        "union",
        "unique",
        "update",
        "using",
        "values",
        "view",
        "when",
        "where",
        "with",
        "ilike",
        "conflict",
        "do",
        "nothing",
        "recursive",
    ],
    constants: &["true", "false", "null"],
    types: &[
        "int",
        "integer",
        "bigint",
        "smallint",
        "serial",
        "bigserial",
        "text",
        "varchar",
        "char",
        "boolean",
        "bool",
        "date",
        "time",
        "timestamp",
        "timestamptz",
        "numeric",
        "decimal",
        "real",
        "float",
        "double",
        "uuid",
        "json",
        "jsonb",
        "bytea",
        "blob",
    ],
    line_comment: "--",
    block_comments: true,
};

/// The first character of `s`, or a space past the end, so lookahead needs no checks.
fn first(s: &str) -> char {
    s.chars().next().unwrap_or(' ')
}

/// The length of the leading run of characters that satisfy `keep`.
fn run_len(s: &str, keep: impl Fn(char) -> bool) -> usize {
    s.find(|c: char| !keep(c)).unwrap_or(s.len())
}

/// The length of the line at the start of `s`, without its line break.
fn line_len(s: &str) -> usize {
    s.find('\n').unwrap_or(s.len())
}

/// The length of `s` up to and including `end`, or all of `s` when it never comes.
fn until(s: &str, from: usize, end: &str) -> usize {
    s.get(from..)
        .and_then(|rest| rest.find(end))
        .map_or(s.len(), |index| from + index + end.len())
}

/// The length of a string at the start of `s` that opens with `open` and closes with
/// `close`. A backslash escapes the next character when `escapes` is set. A string that
/// cannot span lines ends before a line break, and an unclosed one runs to the end.
fn quoted(s: &str, open: usize, close: &str, escapes: bool, multiline: bool) -> usize {
    let mut chars = s.char_indices().skip_while(|&(index, _)| index < open);
    while let Some((index, c)) = chars.next() {
        if escapes && c == '\\' {
            chars.next();
        } else if c == '\n' && !multiline {
            return index;
        } else if s[index..].starts_with(close) {
            return index + close.len();
        }
    }
    s.len()
}

/// The length of a number at the start of `s`, with any type suffix such as `u32`.
fn number_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0;
    if s.len() > 1
        && bytes[0] == b'0'
        && matches!(bytes[1], b'x' | b'X' | b'b' | b'B' | b'o' | b'O')
    {
        return 2 + run_len(&s[2..], |c| c.is_ascii_alphanumeric() || c == '_');
    }
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied().unwrap_or(b' ');
        if b.is_ascii_digit() || b == b'_' || (b == b'.' && next.is_ascii_digit()) {
            i += 1;
        } else if matches!(b, b'e' | b'E')
            && (next.is_ascii_digit()
                || (matches!(next, b'+' | b'-')
                    && bytes.get(i + 2).is_some_and(|b| b.is_ascii_digit())))
        {
            i += 2;
        } else {
            break;
        }
    }
    i + run_len(&s[i..], |c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether `c` can start a word.
fn word_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

/// The length of the word at the start of `s`.
fn word_len(s: &str, extra: &[char]) -> usize {
    run_len(s, |c| c.is_alphanumeric() || c == '_' || extra.contains(&c))
}

/// The length of a bracketed run at the start of `s`, such as `{ … }`, including both
/// brackets, and whether it closed. Quoted text is skipped.
fn bracketed(s: &str, open: char, close: char) -> (usize, bool) {
    let mut depth = 0usize;
    let mut chars = s.char_indices();
    while let Some((index, c)) = chars.next() {
        match c {
            '"' | '\'' | '`' => {
                let len = quoted(&s[index..], 1, &c.to_string(), true, c == '`');
                // Skip to the end of the string.
                for _ in s[index + c.len_utf8()..index + len].chars() {
                    chars.next();
                }
            }
            c if c == open => depth += 1,
            c if c == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return (index + 1, true);
                }
            }
            _ => {}
        }
    }
    (s.len(), false)
}

/// Highlights `s[1..len]` as `inner` between the brackets at `s[0]` and, if `closed`,
/// `s[len - 1]`.
fn nested(s: &str, (len, closed): (usize, bool), inner: &Lexer, out: &mut Out, depth: usize) {
    let end = if closed { len - 1 } else { len };
    out.push(None, &s[..1]);
    inner.run(&s[1..end], out, depth + 1);
    out.push(None, &s[end..len]);
}

/// Highlights a code-like language.
fn code(src: &str, syntax: &'static Syntax, out: &mut Out, depth: usize) {
    let flavor = syntax.flavor;
    let js = matches!(flavor, Flavor::Js { .. });
    let jsx = flavor == Flavor::Js { jsx: true };
    // Whether a value may start here, which tells a regular expression or a JSX tag from
    // division or less-than.
    let mut expression = true;
    // The previous word, so `fn name` and `class Name` can color the name.
    let mut previous = "";
    // Whether a shell command name may start here.
    let mut command = true;
    let mut i = 0;

    while i < src.len() {
        let rest = &src[i..];
        let c = first(rest);
        let at_line_start = || {
            src[..i]
                .rsplit('\n')
                .next()
                .is_none_or(|line| line.trim().is_empty())
        };

        if c.is_whitespace() {
            let len = run_len(rest, char::is_whitespace);
            if flavor == Flavor::Bash && rest[..len].contains('\n') {
                command = true;
            }
            out.push(None, &rest[..len]);
            i += len;
            continue;
        }

        // Comments. A shell `#` starts a comment only at the start of a word.
        let shell_word_start = src[..i]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || c == ';');
        if rest.starts_with(syntax.line_comment) && (flavor != Flavor::Bash || shell_word_start) {
            let len = line_len(rest);
            out.push(Some(Kind::Comment), &rest[..len]);
            i += len;
            continue;
        }
        if syntax.block_comments && rest.starts_with("/*") {
            let len = until(rest, 2, "*/");
            out.push(Some(Kind::Comment), &rest[..len]);
            i += len;
            continue;
        }

        let mut token: Option<(usize, Option<Kind>)> = None;
        match flavor {
            Flavor::Rust => {
                if rest.starts_with("#[") || rest.starts_with("#![") {
                    token = Some((bracketed(rest, '[', ']').0, Some(Kind::Meta)));
                } else if let Some(len) = rust_raw_string(rest) {
                    token = Some((len, Some(Kind::String)));
                } else if rest.starts_with("b\"") || rest.starts_with("c\"") {
                    token = Some((quoted(rest, 2, "\"", true, true), Some(Kind::String)));
                } else if rest.starts_with("b'") {
                    token = Some((quoted(rest, 2, "'", true, false), Some(Kind::String)));
                } else if c == '"' {
                    token = Some((quoted(rest, 1, "\"", true, true), Some(Kind::String)));
                } else if c == '\'' {
                    token = Some(match rust_char(rest) {
                        Some(len) => (len, Some(Kind::String)),
                        // A lifetime or a label.
                        None => (1 + word_len(&rest[1..], &[]), Some(Kind::Variable)),
                    });
                }
            }
            Flavor::Python => {
                let prefix = run_len(rest, |c| "rRbBfFuU".contains(c));
                let after = &rest[prefix.min(2)..];
                if prefix <= 2 && (after.starts_with('"') || after.starts_with('\'')) {
                    let quote = &after[..1];
                    let start = prefix.min(2);
                    let triple = quote.repeat(3);
                    let raw = rest[..start].contains(['r', 'R']);
                    let len = if after.starts_with(&triple) {
                        quoted(rest, start + 3, &triple, !raw, true)
                    } else {
                        quoted(rest, start + 1, quote, !raw, false)
                    };
                    token = Some((len, Some(Kind::String)));
                } else if c == '@' && at_line_start() {
                    let len =
                        1 + run_len(&rest[1..], |c| c.is_alphanumeric() || c == '_' || c == '.');
                    token = Some((len, Some(Kind::Function)));
                }
            }
            Flavor::Js { .. } => {
                if c == '`' {
                    template(src, syntax, out, depth, &mut i);
                    expression = false;
                    continue;
                }
                if c == '"' || c == '\'' {
                    token = Some((quoted(rest, 1, &rest[..1], true, false), Some(Kind::String)));
                } else if c == '/' && expression {
                    if let Some(len) = regex_literal(rest) {
                        token = Some((len, Some(Kind::String)));
                    }
                } else if c == '<' && jsx && expression {
                    let next = first(&rest[1..]);
                    if next.is_alphabetic() || next == '>' {
                        let len = jsx_element(rest, syntax, out, depth);
                        i += len;
                        expression = false;
                        continue;
                    }
                } else if c == '@' {
                    let len = 1 + word_len(&rest[1..], &['.']);
                    token = Some((len, Some(Kind::Function)));
                }
            }
            Flavor::Go => {
                if c == '`' {
                    token = Some((quoted(rest, 1, "`", false, true), Some(Kind::String)));
                } else if c == '"' || c == '\'' {
                    token = Some((quoted(rest, 1, &rest[..1], true, false), Some(Kind::String)));
                }
            }
            Flavor::C => {
                if c == '#' && at_line_start() {
                    token = Some((line_len(rest), Some(Kind::Meta)));
                } else if c == '@' && first(&rest[1..]).is_alphabetic() {
                    // A Java annotation.
                    token = Some((1 + word_len(&rest[1..], &['.']), Some(Kind::Meta)));
                } else if (c == '@' || c == '$') && rest[1..].starts_with('"') {
                    // A C# verbatim or interpolated string.
                    token = Some((quoted(rest, 2, "\"", c == '$', true), Some(Kind::String)));
                } else if c == '"' || c == '\'' {
                    token = Some((quoted(rest, 1, &rest[..1], true, false), Some(Kind::String)));
                }
            }
            Flavor::Bash => {
                if c == '"' {
                    let len = quoted(rest, 1, "\"", true, true);
                    shell_string(&rest[..len], out);
                    i += len;
                    command = false;
                    continue;
                } else if c == '\'' {
                    token = Some((quoted(rest, 1, "'", false, true), Some(Kind::String)));
                } else if c == '$' {
                    let next = first(&rest[1..]);
                    if next == '{' {
                        token = Some((bracketed(&rest[1..], '{', '}').0 + 1, Some(Kind::Variable)));
                    } else if next == '(' {
                        // Command substitution: the command inside is a command.
                        out.push(None, &rest[..2]);
                        i += 2;
                        command = true;
                        continue;
                    } else if word_start(next) {
                        token = Some((1 + word_len(&rest[1..], &[]), Some(Kind::Variable)));
                    } else if next.is_ascii_digit() || "@#?*!$-".contains(next) {
                        token = Some((1 + next.len_utf8(), Some(Kind::Variable)));
                    } else if at_line_start() && next == ' ' {
                        // A prompt in a console session, before a command.
                        out.push(Some(Kind::Meta), "$");
                        i += 1;
                        command = true;
                        continue;
                    }
                }
            }
            Flavor::Sql => {
                if c == '\'' {
                    token = Some((quoted(rest, 1, "'", true, true), Some(Kind::String)));
                } else if c == '"' || c == '`' {
                    token = Some((
                        quoted(rest, 1, &rest[..1], false, false),
                        Some(Kind::Property),
                    ));
                }
            }
        }

        if let Some((len, kind)) = token {
            let len = len.max(c.len_utf8());
            out.push(kind, &rest[..len]);
            i += len;
            expression = false;
            command = false;
            continue;
        }

        if c.is_ascii_digit() || (c == '.' && first(&rest[1..]).is_ascii_digit()) {
            let len = number_len(rest).max(1);
            out.push(Some(Kind::Number), &rest[..len]);
            i += len;
            expression = false;
            command = false;
            continue;
        }

        if word_start(c) || (js && c == '$') || (flavor == Flavor::Bash && command && c == '.') {
            let extra: &[char] = match flavor {
                Flavor::Js { .. } => &['$'],
                Flavor::Bash => &['-', '.', '/'],
                _ => &[],
            };
            let len = word_len(rest, extra).max(c.len_utf8());
            let word = &rest[..len];
            let after = rest[len..].trim_start_matches([' ', '\t']);
            let is = |list: &[&str]| {
                if flavor == Flavor::Sql {
                    list.iter().any(|item| item.eq_ignore_ascii_case(word))
                } else {
                    list.contains(&word)
                }
            };
            let mut len = len;
            if flavor == Flavor::Bash && rest[len..].starts_with('=') {
                // An assignment, which a command may follow.
                out.push(Some(Kind::Variable), word);
                i += len;
                continue;
            }
            let kind = if flavor == Flavor::Bash && command && !is(syntax.keywords) {
                Some(Kind::Function)
            } else if is(syntax.keywords) {
                Some(Kind::Keyword)
            } else if is(syntax.constants) {
                Some(Kind::Number)
            } else if is(syntax.types) {
                Some(Kind::Type)
            } else if flavor == Flavor::Rust && after.starts_with('!') && !after.starts_with("!=") {
                // A macro: the `!` is part of its name.
                if rest[len..].starts_with('!') {
                    len += 1;
                }
                Some(Kind::Function)
            } else if matches!(previous, "fn" | "def" | "function" | "func") {
                Some(Kind::Function)
            } else if matches!(
                previous,
                "class" | "struct" | "enum" | "trait" | "interface" | "type" | "impl" | "union"
            ) {
                Some(Kind::Type)
            } else if after.starts_with('(')
                || (flavor == Flavor::Rust && after.starts_with("::<"))
                || generic_call(after)
            {
                Some(Kind::Function)
            } else if flavor == Flavor::Bash || flavor == Flavor::Sql {
                None
            } else if word.len() > 1
                && word
                    .chars()
                    .all(|c| c.is_uppercase() || c.is_ascii_digit() || c == '_')
                && word.chars().any(char::is_alphabetic)
            {
                Some(Kind::Number)
            } else if first(word).is_uppercase() {
                Some(Kind::Type)
            } else {
                None
            };
            out.push(kind, &rest[..len]);
            i += len;
            expression = kind == Some(Kind::Keyword) && !matches!(word, "this" | "super" | "self");
            previous = word;
            command = flavor == Flavor::Bash && kind == Some(Kind::Keyword) && word != "in";
            continue;
        }

        // Punctuation and operators.
        let len = c.len_utf8();
        out.push(None, &rest[..len]);
        i += len;
        expression = !matches!(c, ')' | ']');
        previous = "";
        if flavor == Flavor::Bash {
            command = matches!(c, '|' | '&' | ';' | '(' | '`' | '{');
        }
    }
}

/// Whether `s` starts with type arguments followed by a call, as in `useState<number>(`.
fn generic_call(s: &str) -> bool {
    if !s.starts_with('<') {
        return false;
    }
    let mut depth = 0usize;
    for (index, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return s[index + 1..].starts_with('(');
                }
            }
            c if c.is_alphanumeric() || " _,.[]{}|&:;?'\"".contains(c) => {}
            _ => return false,
        }
    }
    false
}

/// Highlights a double-quoted shell string, with the variables in it.
fn shell_string(s: &str, out: &mut Out) {
    let mut start = 0;
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        let c = first(rest);
        if c == '\\' {
            i += 1 + first(&rest[1..]).len_utf8().min(rest.len() - 1);
            continue;
        }
        let len = if c != '$' || i == 0 {
            0
        } else if first(&rest[1..]) == '{' {
            1 + bracketed(&rest[1..], '{', '}').0
        } else if word_start(first(&rest[1..])) {
            1 + word_len(&rest[1..], &[])
        } else if first(&rest[1..]).is_ascii_digit() || "@#?*!$".contains(first(&rest[1..])) {
            2
        } else {
            0
        };
        // A variable never takes the closing quote.
        let len = len.min(s.len() - i - usize::from(s.ends_with('"') && s.len() > 1));
        if len > 1 {
            out.push(Some(Kind::String), &s[start..i]);
            out.push(Some(Kind::Variable), &rest[..len]);
            i += len;
            start = i;
        } else {
            i += c.len_utf8();
        }
    }
    out.push(Some(Kind::String), &s[start..]);
}

/// The length of a Rust raw string such as `r#"…"#` or `br"…"` at the start of `s`.
fn rust_raw_string(s: &str) -> Option<usize> {
    let after = s
        .strip_prefix("br")
        .or_else(|| s.strip_prefix("cr"))
        .or_else(|| s.strip_prefix('r'))?;
    let hashes = run_len(after, |c| c == '#');
    if !after[hashes..].starts_with('"') {
        return None;
    }
    let open = s.len() - after.len() + hashes + 1;
    let close = format!("\"{}", "#".repeat(hashes));
    Some(quoted(s, open, &close, false, true))
}

/// The length of a Rust character literal such as `'a'` or `'\n'` at the start of `s`,
/// or `None` when the quote starts a lifetime.
fn rust_char(s: &str) -> Option<usize> {
    let mut chars = s.char_indices().skip(1);
    let (_, c) = chars.next()?;
    if c == '\\' {
        return Some(quoted(s, 1, "'", true, false));
    }
    let (index, next) = chars.next()?;
    (next == '\'').then_some(index + 1)
}

/// The length of a regular expression literal at the start of `s`, flags included, or
/// `None` when the line has no closing slash.
fn regex_literal(s: &str) -> Option<usize> {
    if s.starts_with("//") || s.starts_with("/*") {
        return None;
    }
    let mut in_class = false;
    let mut chars = s.char_indices().skip(1);
    while let Some((index, c)) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '\n' => return None,
            '[' => in_class = true,
            ']' => in_class = false,
            '/' if !in_class => return Some(index + 1 + word_len(&s[index + 1..], &[])),
            _ => {}
        }
    }
    None
}

/// Highlights a template literal at the start of `src[*i..]`, with its `${…}`
/// substitutions as code, and moves `i` past it.
fn template(src: &str, syntax: &'static Syntax, out: &mut Out, depth: usize, i: &mut usize) {
    let rest = &src[*i..];
    let mut start = 0;
    let mut chars = rest.char_indices().skip(1);
    while let Some((index, c)) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '`' => {
                out.push(Some(Kind::String), &rest[start..=index]);
                *i += index + 1;
                return;
            }
            '$' if rest[index + 1..].starts_with('{') => {
                out.push(Some(Kind::String), &rest[start..index]);
                out.push(None, "$");
                let (len, closed) = bracketed(&rest[index + 1..], '{', '}');
                nested(
                    &rest[index + 1..],
                    (len, closed),
                    &Lexer::Code(syntax),
                    out,
                    depth,
                );
                start = index + 1 + len;
                // Skip what the substitution covered.
                while chars.clone().next().is_some_and(|(next, _)| next < start) {
                    chars.next();
                }
                if start >= rest.len() {
                    *i += rest.len();
                    return;
                }
            }
            _ => {}
        }
    }
    out.push(Some(Kind::String), &rest[start..]);
    *i += rest.len();
}

/// Highlights a JSX element at the start of `s`, with its children, and returns its
/// length.
fn jsx_element(s: &str, syntax: &'static Syntax, out: &mut Out, depth: usize) -> usize {
    let mut open = 0usize;
    let mut i = 0;
    loop {
        let rest = &s[i..];
        if rest.is_empty() {
            return i;
        }
        if first(rest) == '<'
            && (first(&rest[1..]).is_alphabetic() || matches!(first(&rest[1..]), '/' | '>'))
        {
            let (len, change) = jsx_tag(rest, syntax, out, depth);
            i += len;
            match change {
                Tag::Open => open += 1,
                Tag::Close => open = open.saturating_sub(1),
                Tag::SelfClosing => {}
            }
            if open == 0 {
                return i;
            }
        } else if first(rest) == '{' {
            let bracket = bracketed(rest, '{', '}');
            nested(rest, bracket, &Lexer::Code(syntax), out, depth);
            i += bracket.0;
        } else {
            // Text between tags.
            let len = run_len(rest, |c| c != '<' && c != '{').max(first(rest).len_utf8());
            out.push(None, &rest[..len]);
            i += len;
        }
    }
}

/// How a tag changes the depth of the element tree.
enum Tag {
    Open,
    Close,
    SelfClosing,
}

/// Highlights one JSX tag at the start of `s`, attributes included.
fn jsx_tag(s: &str, syntax: &'static Syntax, out: &mut Out, depth: usize) -> (usize, Tag) {
    let closing = s.starts_with("</");
    let mut i = if closing { 2 } else { 1 };
    out.push(None, &s[..i]);
    let name = run_len(&s[i..], |c| {
        c.is_alphanumeric() || matches!(c, '_' | '.' | '-' | ':')
    });
    out.push(Some(Kind::Tag), &s[i..i + name]);
    i += name;
    while i < s.len() {
        let rest = &s[i..];
        let c = first(rest);
        if rest.starts_with("/>") {
            out.push(None, "/>");
            return (i + 2, Tag::SelfClosing);
        }
        if c == '>' {
            out.push(None, ">");
            return (i + 1, if closing { Tag::Close } else { Tag::Open });
        }
        let len = if c == '{' {
            let bracket = bracketed(rest, '{', '}');
            nested(rest, bracket, &Lexer::Code(syntax), out, depth);
            i += bracket.0;
            continue;
        } else if c == '"' || c == '\'' {
            let len = quoted(rest, 1, &rest[..1], false, true);
            out.push(Some(Kind::String), &rest[..len]);
            len
        } else if word_start(c) {
            let len = run_len(rest, |c| {
                c.is_alphanumeric() || matches!(c, '_' | '-' | ':')
            });
            out.push(Some(Kind::Property), &rest[..len]);
            len
        } else {
            out.push(None, &rest[..c.len_utf8()]);
            c.len_utf8()
        };
        i += len;
    }
    (s.len(), Tag::SelfClosing)
}

/// Highlights JSON, with the comments JSONC allows. Keys are properties.
fn json(src: &str, out: &mut Out) {
    let mut i = 0;
    while i < src.len() {
        let rest = &src[i..];
        let c = first(rest);
        let (len, kind) = if rest.starts_with("//") {
            (line_len(rest), Some(Kind::Comment))
        } else if rest.starts_with("/*") {
            (until(rest, 2, "*/"), Some(Kind::Comment))
        } else if c == '"' || c == '\'' {
            let len = quoted(rest, 1, &rest[..1], true, false);
            let key = rest[len..].trim_start().starts_with(':');
            (len, Some(if key { Kind::Property } else { Kind::String }))
        } else if c.is_ascii_digit() || c == '-' && first(&rest[1..]).is_ascii_digit() {
            (
                usize::from(c == '-') + number_len(&rest[usize::from(c == '-')..]),
                Some(Kind::Number),
            )
        } else if word_start(c) {
            let len = word_len(rest, &[]);
            let key = rest[len..].trim_start().starts_with(':');
            (len, Some(if key { Kind::Property } else { Kind::Number }))
        } else {
            (
                run_len(rest, |c| {
                    !(c == '"'
                        || c == '\''
                        || c == '/'
                        || c == '-'
                        || c.is_alphanumeric()
                        || c == '_')
                })
                .max(c.len_utf8()),
                None,
            )
        };
        out.push(kind, &rest[..len]);
        i += len;
    }
}

/// Highlights YAML line by line: keys, scalars, comments, anchors and block text.
fn yaml(src: &str, out: &mut Out) {
    // The indentation of the key that opened a `|` or `>` block, while inside it.
    let mut block: Option<usize> = None;
    for line in src.split_inclusive('\n') {
        let indent = run_len(line, |c| c == ' ' || c == '\t');
        let content = line[indent..].trim_end_matches(['\n', '\r']);
        if let Some(parent) = block {
            if content.is_empty() || indent > parent {
                out.push(Some(Kind::String), line);
                continue;
            }
            block = None;
        }
        out.push(None, &line[..indent]);
        let mut rest = &line[indent..];
        if content == "---" || content == "..." {
            out.push(Some(Kind::Meta), rest);
            continue;
        }
        while let Some(after) = rest.strip_prefix("- ") {
            out.push(None, "- ");
            rest = after;
        }
        // A key: a plain or quoted scalar followed by a colon and a space or the end.
        let key_len = if rest.starts_with(['&', '*', '{', '[', '!', '|', '>', '%', '@', '`']) {
            0
        } else if rest.starts_with('"') || rest.starts_with('\'') {
            quoted(rest, 1, &rest[..1], rest.starts_with('"'), false)
        } else {
            yaml_key_len(rest)
        };
        if key_len > 0
            && rest[key_len..].starts_with(':')
            && first(&rest[key_len + 1..]).is_whitespace()
            && !rest.starts_with('#')
        {
            out.push(Some(Kind::Property), &rest[..key_len]);
            out.push(None, ":");
            rest = &rest[key_len + 1..];
        }
        let value = rest.trim();
        if value.starts_with('|') || value.starts_with('>') {
            block = Some(indent);
        }
        yaml_value(rest, out);
    }
}

/// The length of a plain YAML key at the start of `s`, before its colon, or 0.
fn yaml_key_len(s: &str) -> usize {
    for (index, c) in s.char_indices() {
        if c == ':' && first(&s[index + 1..]).is_whitespace() {
            return index;
        }
        if c == '\n' || c == '#' && index > 0 && s[..index].ends_with(' ') {
            return 0;
        }
    }
    0
}

/// Highlights the value part of a YAML line.
fn yaml_value(s: &str, out: &mut Out) {
    let lead = run_len(s, |c| c == ' ' || c == '\t');
    out.push(None, &s[..lead]);
    let s = &s[lead..];
    // An anchor or alias, then whatever follows it.
    if s.starts_with(['&', '*']) {
        let len = 1 + run_len(&s[1..], |c| !c.is_whitespace());
        out.push(Some(Kind::Variable), &s[..len]);
        yaml_value(&s[len..], out);
        return;
    }
    let body_len = s.trim_end_matches(['\n', '\r']).len();
    let (body, end) = s.split_at(body_len);
    // A comment starts at a `#` after a space, outside quotes.
    let comment = if body.starts_with('#') {
        Some(0)
    } else if body.starts_with('"') || body.starts_with('\'') {
        let len = quoted(body, 1, &body[..1], body.starts_with('"'), false);
        body[len..].find(" #").map(|index| len + index + 1)
    } else {
        body.find(" #").map(|index| index + 1)
    };
    let (value, comment) = body.split_at(comment.unwrap_or(body.len()));
    let trimmed = value.trim_end();
    let kind = match trimmed {
        "" | "|" | ">" | "|-" | ">-" | "|+" | ">+" => None,
        "true" | "false" | "yes" | "no" | "on" | "off" | "null" | "~" | "True" | "False"
        | "Null" | "NULL" => Some(Kind::Number),
        _ if trimmed.starts_with('!') => Some(Kind::Meta),
        _ if trimmed.starts_with(['{', '[']) => None,
        _ if number_len(trimmed.trim_start_matches(['-', '+']))
            == trimmed.trim_start_matches(['-', '+']).len()
            && first(trimmed.trim_start_matches(['-', '+'])).is_ascii_digit() =>
        {
            Some(Kind::Number)
        }
        _ => Some(Kind::String),
    };
    if trimmed.starts_with(['{', '[']) {
        // A flow collection reads like JSON.
        json(value, out);
    } else {
        out.push(kind, trimmed);
        out.push(None, &value[trimmed.len()..]);
    }
    out.push(Some(Kind::Comment), comment);
    out.push(None, end);
}

/// Highlights TOML: tables, keys, strings, numbers, dates and comments.
fn toml(src: &str, out: &mut Out) {
    // Brackets opened in values: `[` for arrays, `{` for inline tables.
    let mut brackets: Vec<char> = Vec::new();
    let mut key = true;
    let mut i = 0;
    while i < src.len() {
        let rest = &src[i..];
        let c = first(rest);
        let at_line_start = src[..i]
            .rsplit('\n')
            .next()
            .is_none_or(|line| line.trim().is_empty());
        let (len, kind) = if c == '\n' {
            if brackets.is_empty() {
                key = true;
            }
            (1, None)
        } else if c.is_whitespace() {
            (run_len(rest, |c| c.is_whitespace() && c != '\n'), None)
        } else if c == '#' {
            (line_len(rest), Some(Kind::Comment))
        } else if c == '[' && at_line_start && brackets.is_empty() {
            let close = if rest.starts_with("[[") { "]]" } else { "]" };
            (
                until(rest, 1, close).min(line_len(rest)).max(1),
                Some(Kind::Type),
            )
        } else if rest.starts_with("\"\"\"") || rest.starts_with("'''") {
            (
                quoted(rest, 3, &rest[..3], c == '"', true),
                Some(Kind::String),
            )
        } else if c == '"' || c == '\'' {
            let len = quoted(rest, 1, &rest[..1], c == '"', false);
            (len, Some(if key { Kind::Property } else { Kind::String }))
        } else if key && (c.is_alphanumeric() || c == '_' || c == '-') {
            (
                run_len(rest, |c| c.is_alphanumeric() || matches!(c, '_' | '-')),
                Some(Kind::Property),
            )
        } else if c.is_ascii_digit()
            || (matches!(c, '+' | '-') && first(&rest[1..]).is_ascii_digit())
        {
            // Numbers and dates such as `1979-05-27T07:32:00Z`.
            (
                1 + run_len(&rest[1..], |c| {
                    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-' | '+')
                }),
                Some(Kind::Number),
            )
        } else if word_start(c) {
            let len = word_len(rest, &[]);
            let constant = matches!(&rest[..len], "true" | "false" | "inf" | "nan");
            (len, constant.then_some(Kind::Number))
        } else {
            match c {
                '=' => key = false,
                '[' => brackets.push('['),
                '{' => {
                    brackets.push('{');
                    key = true;
                }
                ']' | '}' => {
                    brackets.pop();
                }
                ',' => key = brackets.last() == Some(&'{'),
                _ => {}
            }
            (c.len_utf8(), None)
        };
        out.push(kind, &rest[..len]);
        i += len;
    }
}

/// Highlights HTML and XML, with the code inside `<script>` and `<style>` elements.
fn html(src: &str, out: &mut Out, depth: usize) {
    let mut i = 0;
    while i < src.len() {
        let rest = &src[i..];
        let c = first(rest);
        if rest.starts_with("<!--") {
            let len = until(rest, 4, "-->");
            out.push(Some(Kind::Comment), &rest[..len]);
            i += len;
        } else if rest.starts_with("<!") || rest.starts_with("<?") {
            let len = until(rest, 2, ">");
            out.push(Some(Kind::Meta), &rest[..len]);
            i += len;
        } else if c == '<'
            && (first(&rest[1..]).is_alphabetic()
                || (first(&rest[1..]) == '/' && first(&rest[2..]).is_alphabetic()))
        {
            let closing = rest.starts_with("</");
            let open = if closing { 2 } else { 1 };
            out.push(None, &rest[..open]);
            let name_len = run_len(&rest[open..], |c| {
                c.is_alphanumeric() || matches!(c, '-' | ':' | '_' | '.')
            });
            let name = &rest[open..open + name_len];
            out.push(Some(Kind::Tag), name);
            let mut j = open + name_len;
            let mut self_closing = false;
            // Attributes, up to the end of the tag.
            while j < rest.len() {
                let tail = &rest[j..];
                let t = first(tail);
                if tail.starts_with("/>") {
                    out.push(None, "/>");
                    j += 2;
                    self_closing = true;
                    break;
                }
                if t == '>' {
                    out.push(None, ">");
                    j += 1;
                    break;
                }
                let (len, kind) = if t == '"' || t == '\'' {
                    (quoted(tail, 1, &tail[..1], false, true), Some(Kind::String))
                } else if t.is_whitespace() || t == '=' || t == '/' {
                    (t.len_utf8(), None)
                } else if rest[..j].ends_with('=') {
                    // An unquoted value.
                    (
                        run_len(tail, |c| !c.is_whitespace() && c != '>'),
                        Some(Kind::String),
                    )
                } else {
                    (
                        run_len(tail, |c| {
                            !c.is_whitespace() && !matches!(c, '=' | '>' | '/' | '"' | '\'')
                        })
                        .max(t.len_utf8()),
                        Some(Kind::Property),
                    )
                };
                out.push(kind, &tail[..len.max(1).min(tail.len())]);
                j += len.max(1).min(tail.len());
            }
            i += j;
            // The body of a script or style, up to its closing tag.
            let embedded = match name.to_ascii_lowercase().as_str() {
                "script" => Some(Lexer::Code(&TSX)),
                "style" => Some(Lexer::Css {
                    line_comments: false,
                }),
                _ => None,
            };
            if let (Some(lexer), false, false) = (embedded, closing, self_closing) {
                let body = &src[i..];
                let end = body
                    .to_ascii_lowercase()
                    .find(&format!("</{}", name.to_ascii_lowercase()))
                    .unwrap_or(body.len());
                lexer.run(&body[..end], out, depth + 1);
                i += end;
            }
        } else if c == '&' {
            let len = 1 + run_len(&rest[1..], |c| c.is_alphanumeric() || c == '#');
            if rest[len..].starts_with(';') {
                out.push(Some(Kind::Number), &rest[..=len]);
                i += len + 1;
            } else {
                out.push(None, "&");
                i += 1;
            }
        } else {
            let len = run_len(rest, |c| c != '<' && c != '&').max(c.len_utf8());
            out.push(None, &rest[..len]);
            i += len;
        }
    }
}

/// Highlights CSS: selectors, properties, values, at-rules and comments.
fn css(src: &str, line_comments: bool, out: &mut Out) {
    let mut depth = 0usize;
    // Whether a declaration's value is being read.
    let mut value = false;
    let mut i = 0;
    while i < src.len() {
        let rest = &src[i..];
        let c = first(rest);
        let (len, kind) = if rest.starts_with("/*") {
            (until(rest, 2, "*/"), Some(Kind::Comment))
        } else if line_comments && rest.starts_with("//") {
            (line_len(rest), Some(Kind::Comment))
        } else if c == '"' || c == '\'' {
            (quoted(rest, 1, &rest[..1], true, false), Some(Kind::String))
        } else if c == '@' {
            (1 + word_len(&rest[1..], &['-']), Some(Kind::Keyword))
        } else if rest.starts_with("!important") {
            (10, Some(Kind::Keyword))
        } else if value && c == '#' {
            (1 + word_len(&rest[1..], &[]), Some(Kind::Number))
        } else if value
            && (c.is_ascii_digit()
                || (matches!(c, '.' | '-' | '+') && first(&rest[1..]).is_ascii_digit()))
        {
            let sign = usize::from(matches!(c, '-' | '+'));
            let len = sign + number_len(&rest[sign..]);
            (
                len + usize::from(rest[len..].starts_with('%')),
                Some(Kind::Number),
            )
        } else if !value && (c == '.' || c == '#') && word_start(first(&rest[1..])) {
            (1 + word_len(&rest[1..], &['-']), Some(Kind::Type))
        } else if !value && c == ':' && (word_start(first(&rest[1..])) || first(&rest[1..]) == ':')
        {
            let colons = run_len(rest, |c| c == ':');
            (
                colons + word_len(&rest[colons..], &['-']),
                Some(Kind::Function),
            )
        } else if word_start(c)
            || c == '-' && (word_start(first(&rest[1..])) || first(&rest[1..]) == '-')
        {
            let len = word_len(rest, &['-']);
            let after = rest[len..].trim_start();
            let kind = if value {
                after.starts_with('(').then_some(Kind::Function)
            } else if depth > 0 && after.starts_with(':') {
                Some(Kind::Property)
            } else {
                Some(Kind::Tag)
            };
            (len, kind)
        } else {
            match c {
                '{' => {
                    depth += 1;
                    value = false;
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    value = false;
                }
                ':' if depth > 0 => value = true,
                ';' => value = false,
                _ => {}
            }
            (c.len_utf8(), None)
        };
        let len = len.max(c.len_utf8()).min(rest.len());
        out.push(kind, &rest[..len]);
        i += len;
    }
}

/// Highlights a unified diff, line by line.
fn diff(src: &str, out: &mut Out) {
    for line in src.split_inclusive('\n') {
        let kind = if line.starts_with("+++")
            || line.starts_with("---")
            || line.starts_with("diff ")
            || line.starts_with("index ")
        {
            Some(Kind::Meta)
        } else if line.starts_with("@@") {
            Some(Kind::Function)
        } else if line.starts_with('+') {
            Some(Kind::Inserted)
        } else if line.starts_with('-') {
            Some(Kind::Deleted)
        } else if line.starts_with('\\') {
            Some(Kind::Comment)
        } else {
            None
        };
        let body = line.trim_end_matches('\n');
        out.push(kind, body);
        out.push(None, &line[body.len()..]);
    }
}

#[cfg(test)]
mod tests {
    use super::highlight;

    /// One sample per lexer, with the constructs each one handles.
    const SAMPLES: &[(&str, &str)] = &[
        (
            "rust",
            "/// Docs with <b>.\n#[derive(Debug, \"]\")]\npub async fn send<'a>(cx: &'a Cx) -> Result<u32> {\n    let s = r#\"raw \"q\" </span>\"#;\n    let c = '\\n'; let b = b\"x\";\n    println!(\"{} & {}\", 1_000u32, 0x1f);\n    'outer: loop { break 'outer; }\n    Vec::<u8>::new(); MAX_SIZE /* note */\n}\n",
        ),
        (
            "python",
            "@app.route(\"/\")\ndef greet(name: str = 'x') -> None:\n    \"\"\"Says <hi> & bye.\"\"\"\n    print(f\"Hello {name}!\", rb'\\d+', 3.5e-2)  # comment\n    return None if not name else True\n\nclass Garden(Base):\n    pass\n",
        ),
        (
            "tsx",
            "import { useState, type ReactNode } from \"react\";\n// A card.\ninterface Props { id: number; fallback?: ReactNode }\nexport function Card({ id }: Props) {\n  const [n, setN] = useState<number>(0);\n  const url = `/api/${id}?q=${encodeURIComponent(`a${n}`)}`;\n  const ok = /^[a-z/]+$/i.test(url) && n < 3 && n / 2 > 1;\n  return (\n    <section className=\"card\" onClick={() => setN(n + 1)}>\n      <h2>{n > 0 ? <b>yes</b> : 'no'}</h2>\n      <Icon.Star size={12} /> & text </section>\n  );\n}\n",
        ),
        (
            "js",
            "const total = [1, 2.5, 0x1f].reduce((a, b) => a + b, 0) / 2;\nconst re = /\\/\\*[^*]*\\*\\//g; // not a comment\nlet t = `multi\nline ${total}`;\nconsole.log(typeof t === 'string', null ?? undefined);\n",
        ),
        (
            "go",
            "package main\n\nimport \"fmt\"\n\n// Sum adds.\nfunc Sum(xs []int) (total int, err error) {\n\tfor _, x := range xs { total += x }\n\traw := `C:\\path`\n\tfmt.Println(raw, 'x', nil)\n\treturn\n}\n",
        ),
        (
            "c",
            "#include <stdio.h>\n#define MAX 10\nint main(void) {\n    char *s = \"a \\\"b\\\" <c>\"; /* x */\n    printf(\"%d\\n\", MAX); return 0;\n}\n",
        ),
        (
            "csharp",
            "[Serializable]\npublic record Point(int X, int Y);\nvar path = @\"C:\\dir\"; var msg = $\"x {path}\";\n",
        ),
        (
            "java",
            "@Override\npublic String toString() { return \"Point\" + x; }\n",
        ),
        (
            "bash",
            "#!/usr/bin/env bash\n# Deploy <it> & check.\nset -euo pipefail\nexport NAME=\"site-${ENV:-dev}\"\nfor f in *.rs; do echo \"$f\" | wc -l; done\nif [ -n \"$1\" ]; then cargo build --release && ./target/release/app 2>&1; fi\nfiles=$(ls -a) # list\n$ npm install\n",
        ),
        (
            "sql",
            "-- Top customers\nSELECT c.name, COUNT(o.id) AS orders\nFROM customers c\nLEFT JOIN orders o ON o.customer_id = c.id\nWHERE c.created_at > '2024-01-01' AND \"weird col\" <> 'it''s'\nGROUP BY c.name /* block */ LIMIT 10;\n",
        ),
        (
            "json",
            "{\n  \"name\": \"garden\", // jsonc\n  \"version\": 2, \"ratio\": -1.5e3,\n  \"tags\": [\"a</span>\", true, null],\n  /* block */ \"nested\": { \"ok\": false }\n}\n",
        ),
        (
            "yaml",
            "# Config <x>\n---\nname: garden\nversion: 2\nenabled: true\nurl: \"https://x.test/?a=1&b=2\"  # trailing\ntags: [a, \"b\", 3]\nservers:\n  - host: example.com\n    port: 8080\n  - &anchor { host: b }\nscript: |\n  echo \"<hi>\"\n  key: not a key\nnext: *anchor\n",
        ),
        (
            "toml",
            "# Cargo <manifest>\n[package]\nname = \"garden\"\nversion = \"0.1.0\"\n\"quoted key\" = 'lit'\n\n[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nlist = [\n  1, 2.5, -3,\n  true,\n]\nwhen = 1979-05-27T07:32:00Z\ntext = \"\"\"\nmulti & <line>\n\"\"\"\n[[bin]]\n",
        ),
        (
            "html",
            "<!DOCTYPE html>\n<!-- A page & more -->\n<html lang=\"en\">\n<body class=main data-x='1'>\n  <p>Fish &amp; chips &lt;3 & co</p>\n  <img src=\"a.png\" alt=\"\"/>\n  <script>const x = 1 < 2 && \"</div>\"; if (x) { go(); }</script>\n  <style>p { color: #333; margin: 0 4px; }</style>\n</body>\n</html>\n",
        ),
        (
            "css",
            "/* Theme <x> */\n@media (min-width: 640px) {\n  .card > a:hover, #main::before {\n    color: rgb(0 0 0 / 50%) !important;\n    --gap: -0.5rem;\n    content: \"</span>\";\n  }\n}\nbody { margin: 0; font: 14px/1.5 sans-serif }\n",
        ),
        (
            "diff",
            "diff --git a/src/main.rs b/src/main.rs\nindex 1a2b..3c4d 100644\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,3 @@ fn main()\n fn main() {\n-    println!(\"<old>\");\n+    println!(\"new & better\");\n }\n\\ No newline at end of file\n",
        ),
    ];

    /// Removes the highlighter's spans and undoes its escapes, leaving the text the
    /// browser shows. Panics on any other markup, so a leak fails the test.
    fn text(html: &str) -> String {
        let mut rest = html;
        let mut out = String::new();
        let mut open = 0usize;
        while let Some(index) = rest.find('<') {
            out.push_str(&rest[..index]);
            rest = &rest[index..];
            if let Some(after) = rest.strip_prefix("</span>") {
                assert!(open > 0, "unbalanced span in {html}");
                open -= 1;
                rest = after;
            } else if rest.starts_with("<span class=\"tk-") {
                assert_eq!(open, 0, "nested span in {html}");
                open += 1;
                rest = &rest[rest.find('>').unwrap() + 1..];
            } else {
                panic!("unexpected markup in {html}");
            }
        }
        out.push_str(rest);
        assert_eq!(open, 0, "unclosed span in {html}");
        assert!(!out.contains('>'), "unescaped > in {html}");
        let unescaped = out
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'");
        assert!(
            unescaped
                .match_indices('&')
                .all(|(index, _)| unescaped[index..].starts_with("&amp;")),
            "unescaped & in {html}"
        );
        unescaped.replace("&amp;", "&")
    }

    #[test]
    fn keeps_the_text_of_every_sample() {
        for (language, code) in SAMPLES {
            let html = highlight(language, code).unwrap();
            assert_eq!(&text(&html), code, "{language}");
        }
    }

    /// A streaming block shows every prefix of the final code in turn.
    #[test]
    fn keeps_the_text_of_every_prefix() {
        for (language, code) in SAMPLES {
            for (end, _) in code.char_indices() {
                let prefix = &code[..end];
                let html = highlight(language, prefix).unwrap();
                assert_eq!(text(&html), prefix, "{language} cut at {end}");
            }
        }
    }

    #[test]
    fn escapes_markup_in_every_language() {
        let hostile = "</span><script>alert('x')</script> & &amp; \"<img onerror=x>\" ' > <!-- `${'</span>'}` /* </code> */";
        for (language, _) in SAMPLES {
            let html = highlight(language, hostile).unwrap();
            assert!(!html.contains("<script"), "{language}: {html}");
            assert!(!html.contains("<img"), "{language}: {html}");
            assert!(!html.contains("</code>"), "{language}: {html}");
            assert_eq!(text(&html), hostile, "{language}");
        }
    }

    /// Random input from the characters the lexers care about never panics or loses text.
    #[test]
    fn survives_random_input() {
        const PIECES: &[&str] = &[
            "\"", "'", "`", "${", "{", "}", "<", ">", "</", "/>", "/", "*", "/*", "*/", "//", "#",
            "#[", "!", "\\", "\n", " ", "-", "+", ":", "=", "@@", "&", ";", "$", "$(", "r#\"",
            "\"\"\"", "<!--", "<script>", "a", "Z", "fn", "def", "if", "1", "0x", ".5", "é", "日",
            "[", "]", "(", ")", "|", "- ",
        ];
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for (language, _) in SAMPLES {
            for _ in 0..300 {
                let len = next() % 60;
                let code: String = (0..len)
                    .map(|_| PIECES[(next() % PIECES.len() as u64) as usize])
                    .collect();
                let html = highlight(language, &code).unwrap();
                assert_eq!(text(&html), code, "{language}: {code:?}");
            }
        }
    }

    #[test]
    fn deep_nesting_stays_plain_instead_of_overflowing() {
        let jsx = "<a>{".repeat(5_000);
        assert_eq!(text(&highlight("jsx", &jsx).unwrap()), jsx);
        let template = "`${".repeat(5_000);
        assert_eq!(text(&highlight("ts", &template).unwrap()), template);
        let html = "<script><".repeat(2_000);
        assert_eq!(text(&highlight("html", &html).unwrap()), html);
    }

    #[test]
    fn resolves_names_and_aliases() {
        for language in [
            "rust", "rs", "Python", "py", "js", "JSX", "ts", "tsx", "sh", "shell", "console",
            "yml", "yaml", "toml", "json", "html", "xml", "css", "scss", "sql", "go", "diff",
            "patch", "c", "cpp", "java", "cs", " rust ",
        ] {
            assert!(highlight(language, "x").is_some(), "{language}");
        }
        for language in ["", "text", "plaintext", "brainfuck", "rust,ignore"] {
            assert!(highlight(language, "x").is_none(), "{language}");
        }
    }

    /// The HTML of `code` as `language`, for checking how tokens are classed.
    fn spans(language: &str, code: &str) -> String {
        highlight(language, code).unwrap()
    }

    fn has(html: &str, kind: &str, token: &str) -> bool {
        html.contains(&format!("<span class=\"tk-{kind}\">{token}</span>"))
    }

    #[test]
    fn classes_rust() {
        let html = spans("rust", SAMPLES[0].1);
        for (kind, token) in [
            ("comment", "/// Docs with &lt;b&gt;."),
            ("keyword", "pub"),
            ("function", "send"),
            ("type", "Cx"),
            ("variable", "&#39;a"),
            ("function", "println!"),
            ("number", "1_000u32"),
            ("number", "0x1f"),
            ("number", "MAX_SIZE"),
            ("string", "&#39;\\n&#39;"),
        ] {
            assert!(has(&html, kind, token), "{kind} {token}: {html}");
        }
        assert!(html.contains("<span class=\"tk-meta\">#[derive(Debug, &quot;]&quot;)]</span>"));
        assert!(html.contains("tk-string\">r#&quot;raw &quot;q&quot; &lt;/span&gt;&quot;#</span>"));
    }

    #[test]
    fn classes_python() {
        let html = spans("python", SAMPLES[1].1);
        for (kind, token) in [
            ("function", "@app.route"),
            ("keyword", "def"),
            ("function", "greet"),
            ("type", "str"),
            ("string", "&#39;x&#39;"),
            ("number", "None"),
            (
                "string",
                "&quot;&quot;&quot;Says &lt;hi&gt; &amp; bye.&quot;&quot;&quot;",
            ),
            ("string", "rb&#39;\\d+&#39;"),
            ("number", "3.5e-2"),
            ("comment", "# comment"),
            ("type", "Garden"),
        ] {
            assert!(has(&html, kind, token), "{kind} {token}: {html}");
        }
    }

    #[test]
    fn classes_typescript_and_jsx() {
        let html = spans("tsx", SAMPLES[2].1);
        for (kind, token) in [
            ("keyword", "import"),
            ("keyword", "interface"),
            ("type", "Props"),
            ("type", "number"),
            ("function", "useState"),
            ("tag", "section"),
            ("property", "className"),
            ("string", "&quot;card&quot;"),
            ("tag", "b"),
            ("tag", "Icon.Star"),
            ("string", "/^[a-z/]+$/i"),
            ("string", "`/api/"),
        ] {
            assert!(has(&html, kind, token), "{kind} {token}: {html}");
        }
        // `n < 3` and `n / 2` are operators, not a tag or a regular expression.
        assert!(
            html.contains("n &lt; <span class=\"tk-number\">3</span>"),
            "{html}"
        );
        assert!(
            html.contains("n / <span class=\"tk-number\">2</span>"),
            "{html}"
        );
        // Children text stays plain.
        assert!(html.contains("&gt; &amp; text &lt;/"), "{html}");
    }

    #[test]
    fn classes_the_data_and_markup_languages() {
        let json = spans("json", SAMPLES[10].1);
        assert!(has(&json, "property", "&quot;name&quot;"), "{json}");
        assert!(has(&json, "string", "&quot;garden&quot;"), "{json}");
        assert!(has(&json, "number", "-1.5e3"), "{json}");
        assert!(has(&json, "comment", "// jsonc"), "{json}");

        let yaml = spans("yaml", SAMPLES[11].1);
        assert!(has(&yaml, "property", "servers"), "{yaml}");
        assert!(has(&yaml, "property", "host"), "{yaml}");
        assert!(has(&yaml, "number", "8080"), "{yaml}");
        assert!(has(&yaml, "number", "true"), "{yaml}");
        assert!(has(&yaml, "comment", "# trailing"), "{yaml}");
        assert!(yaml.contains("  key: not a key\n</span>"), "{yaml}");
        assert!(has(&yaml, "variable", "&amp;anchor"), "{yaml}");
        assert!(has(&yaml, "variable", "*anchor"), "{yaml}");

        let toml = spans("toml", SAMPLES[12].1);
        assert!(has(&toml, "type", "[package]"), "{toml}");
        assert!(has(&toml, "type", "[[bin]]"), "{toml}");
        assert!(has(&toml, "property", "serde"), "{toml}");
        assert!(has(&toml, "property", "features"), "{toml}");
        assert!(has(&toml, "string", "&quot;derive&quot;"), "{toml}");
        assert!(has(&toml, "number", "1979-05-27T07:32:00Z"), "{toml}");
        assert!(has(&toml, "number", "true"), "{toml}");

        let html = spans("html", SAMPLES[13].1);
        assert!(has(&html, "tag", "body"), "{html}");
        assert!(has(&html, "property", "class"), "{html}");
        assert!(has(&html, "string", "main"), "{html}");
        assert!(has(&html, "number", "&amp;amp;"), "{html}");
        assert!(has(&html, "keyword", "const"), "{html}");
        assert!(has(&html, "string", "&quot;&lt;/div&gt;&quot;"), "{html}");
        assert!(has(&html, "property", "color"), "{html}");

        let css = spans("css", SAMPLES[14].1);
        assert!(has(&css, "keyword", "@media"), "{css}");
        assert!(has(&css, "type", ".card"), "{css}");
        assert!(has(&css, "function", ":hover"), "{css}");
        assert!(has(&css, "property", "color"), "{css}");
        assert!(has(&css, "property", "--gap"), "{css}");
        assert!(has(&css, "number", "-0.5rem"), "{css}");
        assert!(has(&css, "number", "50%"), "{css}");
        assert!(has(&css, "function", "rgb"), "{css}");
        assert!(has(&css, "keyword", "!important"), "{css}");
    }

    #[test]
    fn classes_shell_sql_and_diffs() {
        let bash = spans("bash", SAMPLES[8].1);
        for (kind, token) in [
            ("comment", "#!/usr/bin/env bash"),
            ("function", "set"),
            ("keyword", "export"),
            ("variable", "${ENV:-dev}"),
            ("keyword", "for"),
            ("function", "echo"),
            ("function", "wc"),
            ("variable", "$1"),
            ("function", "cargo"),
            ("function", "./target/release/app"),
            ("function", "ls"),
            ("comment", "# list"),
            ("meta", "$"),
            ("function", "npm"),
            ("variable", "files"),
        ] {
            assert!(has(&bash, kind, token), "{kind} {token}: {bash}");
        }
        assert!(!has(&bash, "function", "install"), "{bash}");

        let sql = spans("sql", SAMPLES[9].1);
        assert!(has(&sql, "keyword", "SELECT"), "{sql}");
        assert!(has(&sql, "keyword", "LEFT"), "{sql}");
        assert!(has(&sql, "function", "COUNT"), "{sql}");
        assert!(has(&sql, "string", "&#39;it&#39;&#39;s&#39;"), "{sql}");
        assert!(has(&sql, "property", "&quot;weird col&quot;"), "{sql}");
        assert!(has(&sql, "comment", "-- Top customers"), "{sql}");

        let diff = spans("diff", SAMPLES[15].1);
        assert!(has(&diff, "meta", "+++ b/src/main.rs"), "{diff}");
        assert!(
            has(&diff, "function", "@@ -1,3 +1,3 @@ fn main()"),
            "{diff}"
        );
        assert!(
            has(&diff, "deleted", "-    println!(&quot;&lt;old&gt;&quot;);"),
            "{diff}"
        );
        assert!(
            has(
                &diff,
                "inserted",
                "+    println!(&quot;new &amp; better&quot;);"
            ),
            "{diff}"
        );
    }

    /// A string or comment that is still streaming runs to the end without swallowing
    /// the lines before it.
    #[test]
    fn highlights_unfinished_code_sensibly() {
        let html = spans("python", "x = 1\ns = \"unfinished");
        assert!(has(&html, "number", "1"), "{html}");
        assert!(has(&html, "string", "&quot;unfinished"), "{html}");
        let html = spans("ts", "const a = 'one\nconst b = 2");
        // A single-quoted string ends at the line break.
        assert!(has(&html, "keyword", "const"), "{html}");
        assert!(html.matches("tk-keyword").count() == 2, "{html}");
        let html = spans("rust", "fn main() { /* still");
        assert!(has(&html, "comment", "/* still"), "{html}");
    }
}
