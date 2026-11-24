//! A small Rust syntax highlighter for the docs' code panels.
//!
//! It splits source into tokens and wraps the interesting ones in `<span class="tok-*">`
//! elements, which `styles.css` colors. It is a lexer, not a parser: good enough for
//! the example and component sources shown on the site, and with no dependencies.
//! Everything else is escaped, so the output is safe to embed.

/// The kind of a highlighted token, which is its CSS class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Macro,
    Type,
    Attribute,
    Lifetime,
}

impl Kind {
    fn class(self) -> &'static str {
        match self {
            Self::Keyword => "tok-keyword",
            Self::String => "tok-string",
            Self::Comment => "tok-comment",
            Self::Number => "tok-number",
            Self::Function => "tok-function",
            Self::Macro => "tok-macro",
            Self::Type => "tok-type",
            Self::Attribute => "tok-attribute",
            Self::Lifetime => "tok-lifetime",
        }
    }
}

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
];

/// Returns `source` as HTML with Rust tokens wrapped in classed spans.
pub fn rust(source: &str) -> String {
    let mut out = String::with_capacity(source.len() * 2);
    let bytes = source.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let rest = &source[i..];
        let c = rest.chars().next().unwrap();

        // Comments, including doc comments.
        if rest.starts_with("//") {
            let end = rest.find('\n').unwrap_or(rest.len());
            push(&mut out, Some(Kind::Comment), &rest[..end]);
            i += end;
            continue;
        }
        if let Some(body) = rest.strip_prefix("/*") {
            let end = body.find("*/").map_or(rest.len(), |end| end + 4);
            push(&mut out, Some(Kind::Comment), &rest[..end]);
            i += end;
            continue;
        }

        // Attributes, up to the matching bracket.
        if rest.starts_with("#[") || rest.starts_with("#![") {
            let end = matching_bracket(rest).unwrap_or(rest.len());
            push(&mut out, Some(Kind::Attribute), &rest[..end]);
            i += end;
            continue;
        }

        // Raw strings, byte strings, then ordinary strings.
        if let Some(len) = raw_string(rest) {
            push(&mut out, Some(Kind::String), &rest[..len]);
            i += len;
            continue;
        }
        if c == '"' || rest.starts_with("b\"") {
            let start = usize::from(c == 'b');
            let len = quoted(&rest[start..], '"').map_or(rest.len(), |len| len + start);
            push(&mut out, Some(Kind::String), &rest[..len]);
            i += len;
            continue;
        }

        // A character literal, or a lifetime.
        if c == '\'' {
            if let Some(len) = char_literal(rest) {
                push(&mut out, Some(Kind::String), &rest[..len]);
                i += len;
                continue;
            }
            let len = 1 + rest[1..]
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(rest.len() - 1);
            push(&mut out, Some(Kind::Lifetime), &rest[..len]);
            i += len;
            continue;
        }

        if c.is_ascii_digit() {
            let len = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
                .unwrap_or(rest.len());
            // `1..2` is two numbers and a range, not one number.
            let len = rest[..len].find("..").unwrap_or(len);
            push(&mut out, Some(Kind::Number), &rest[..len]);
            i += len;
            continue;
        }

        if c.is_alphabetic() || c == '_' {
            let len = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            let word = &rest[..len];
            let after = rest[len..].trim_start_matches(' ');
            let kind = if KEYWORDS.contains(&word) {
                Some(Kind::Keyword)
            } else if after.starts_with('!') && !after.starts_with("!=") {
                Some(Kind::Macro)
            } else if after.starts_with('(') || after.starts_with("::<") {
                Some(Kind::Function)
            } else if word.starts_with(|c: char| c.is_uppercase()) {
                Some(Kind::Type)
            } else {
                None
            };
            // The `!` belongs to the macro's name.
            let len = if kind == Some(Kind::Macro) && rest[len..].starts_with('!') {
                len + 1
            } else {
                len
            };
            push(&mut out, kind, &rest[..len]);
            i += len;
            continue;
        }

        push(&mut out, None, &rest[..c.len_utf8()]);
        i += c.len_utf8();
    }

    out
}

/// Escapes `text` and appends it, wrapped in a span when it has a kind.
fn push(out: &mut String, kind: Option<Kind>, text: &str) {
    if let Some(kind) = kind {
        out.push_str("<span class=\"");
        out.push_str(kind.class());
        out.push_str("\">");
        escape(out, text);
        out.push_str("</span>");
    } else {
        escape(out, text);
    }
}

fn escape(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
}

/// The length of a quoted literal starting at `rest`, closing quote included.
fn quoted(rest: &str, quote: char) -> Option<usize> {
    let mut escaped = false;
    for (index, c) in rest.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == quote {
            return Some(index + 1);
        }
    }
    None
}

/// The length of a raw string such as `r#"…"#` or `br"…"` at the start of `rest`.
fn raw_string(rest: &str) -> Option<usize> {
    let after_prefix = rest.strip_prefix("br").or_else(|| rest.strip_prefix('r'))?;
    let hashes = after_prefix.len() - after_prefix.trim_start_matches('#').len();
    let body = after_prefix[hashes..].strip_prefix('"')?;
    let closing = format!("\"{}", "#".repeat(hashes));
    let end = body.find(&closing)?;
    Some(rest.len() - body.len() + end + closing.len())
}

/// The length of a character literal such as `'a'` or `'\n'` at the start of `rest`,
/// or `None` when the quote starts a lifetime.
fn char_literal(rest: &str) -> Option<usize> {
    let mut chars = rest.char_indices().skip(1);
    let (_, first) = chars.next()?;
    if first == '\\' {
        return quoted(rest, '\'');
    }
    let (index, second) = chars.next()?;
    (second == '\'').then_some(index + 1)
}

/// The length of the bracketed attribute at the start of `rest`, brackets included.
fn matching_bracket(rest: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;
    for (index, c) in rest.char_indices() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index + 1);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::rust;

    /// Removes the markup, leaving the text the browser would show.
    fn text(html: &str) -> String {
        let mut out = String::new();
        let mut in_tag = false;
        for c in html.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => out.push(c),
                _ => {}
            }
        }
        out.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&amp;", "&")
    }

    #[test]
    fn keeps_the_text_of_every_example() {
        for source in [
            include_str!("main.rs"),
            include_str!("highlight.rs"),
            include_str!("api.rs"),
            include_str!("../../crates/top-prompt/src/components/markdown.rs"),
        ] {
            assert_eq!(text(&rust(source)), source);
        }
    }

    #[test]
    fn classes_tokens() {
        let html = rust("pub async fn send(cx: &Cx) -> Result<u32> { view! { \"a<b\" } 42 }");
        assert!(
            html.contains(r#"<span class="tok-keyword">pub</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="tok-function">send</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="tok-type">Cx</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="tok-macro">view!</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="tok-string">&quot;a&lt;b&quot;</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="tok-number">42</span>"#),
            "{html}"
        );
    }

    #[test]
    fn comments_attributes_lifetimes_and_raw_strings() {
        let html = rust("/// Docs\n#[default(\"]\")]\nfn f<'a>(x: &'a str) { r#\"q\"#; 'c'; }");
        assert!(
            html.contains(r#"<span class="tok-comment">/// Docs</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="tok-attribute">#[default(&quot;]&quot;)]</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="tok-lifetime">'a</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r##"<span class="tok-string">r#&quot;q&quot;#</span>"##),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="tok-string">'c'</span>"#),
            "{html}"
        );
    }

    #[test]
    fn escapes_markup_in_code() {
        let html = rust("let s = \"<script>\"; // </span>");
        assert!(!html.contains("<script>"));
        assert!(!html.contains("// </span>"));
    }
}
