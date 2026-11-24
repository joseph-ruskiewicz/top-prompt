use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd, html};
use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, StaticClass, Unescaped, View, class, component, view},
};

use super::code_block::code_block;

/// Typography for rendered markdown, applied through descendant selectors so the
/// generated elements need no classes of their own. Blocks are spaced by the wrapper's
/// gap.
const PROSE: StaticClass = class!(
    "flex min-w-0 flex-col gap-3 text-sm leading-relaxed text-foreground break-words",
    // Headings.
    "[&_h1]:text-xl [&_h2]:text-lg [&_h3]:text-base",
    "[&_:is(h1,h2,h3,h4,h5,h6)]:leading-snug [&_:is(h1,h2,h3,h4,h5,h6)]:font-semibold",
    "[&>:is(h1,h2,h3)]:mt-2 [&>:is(h1,h2,h3):first-child]:mt-0",
    // Inline text.
    "[&_strong]:font-semibold [&_del]:text-muted-foreground",
    "[&_a]:font-medium [&_a]:underline [&_a]:underline-offset-4",
    "[&_a]:transition-colors [&_a:hover]:text-muted-foreground",
    "[&_:not(pre)>code]:rounded-sm [&_:not(pre)>code]:bg-foreground/5",
    "[&_:not(pre)>code]:px-1 [&_:not(pre)>code]:py-0.5",
    "[&_:not(pre)>code]:font-mono [&_:not(pre)>code]:text-[0.9em]",
    // Lists, including GitHub task lists.
    "[&_ul]:list-disc [&_ol]:list-decimal [&_:is(ul,ol)]:pl-6",
    "[&_li]:my-1 [&_li]:pl-1 [&_li]:marker:text-muted-foreground",
    "[&_li>:is(ul,ol)]:mt-1",
    "[&_li:has(>input)]:list-none [&_li>input]:mr-2 [&_li>input]:-ml-5",
    "[&_li>input]:align-middle [&_li>input]:accent-primary",
    // Block quotes and rules.
    "[&_blockquote]:border-l-2 [&_blockquote]:border-border [&_blockquote]:pl-4",
    "[&_blockquote]:text-muted-foreground",
    "[&_hr]:my-2 [&_hr]:border-border",
    // Tables, styled like the built-in table.
    "[&_table]:w-full [&_table]:border-collapse",
    "[&_tr]:border-b [&_tr]:border-border [&_tbody>tr:last-child]:border-b-0",
    "[&_thead]:bg-foreground/5",
    "[&_:is(th,td)]:px-3 [&_:is(th,td)]:py-2 [&_:is(th,td)]:align-middle",
    "[&_th]:text-left [&_th]:font-medium [&_th]:text-muted-foreground",
    // Images.
    "[&_img]:max-w-full [&_img]:rounded-lg",
);

/// Classes for the scroll container wrapped around each table.
const TABLE_WRAPPER: &str = r#"<div data-slot="markdown-table" class="w-full overflow-x-auto rounded-lg border border-border">"#;

/// Renders markdown on the server, with GitHub-style tables, strikethrough, and task
/// lists.
///
/// **This component needs the `pulldown-cmark` crate**: add it to your app with
/// `cargo add pulldown-cmark --no-default-features --features html`.
///
/// Rendering is safe for untrusted input such as model output. HTML in the source is
/// shown as text instead of being passed through. Links work only for `http`, `https`,
/// and `mailto` URLs and open in a new tab; other links, such as `javascript:` URLs,
/// render as their text. Images load only from `http` and `https` URLs and otherwise
/// render as their alternative text.
///
/// Fenced code blocks render as a [`code_block`] with a copy button. Incomplete
/// markdown, such as an unclosed code fence or half a link while a response streams,
/// renders as far as it goes.
///
/// `attrs` are forwarded to the outer `<div>`. Extra classes are added to its classes.
///
/// ```ignore
/// view! {
///     message_content(markdown(source: &response))
/// }
/// ```
#[component]
pub async fn markdown(
    cx: &Cx,
    /// The markdown text.
    source: &str,
    /// Extra attributes for the outer `<div>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    let blocks = render(source);

    Ok(view! {
        <div data-slot="markdown" class=(class!(PROSE, attrs.remove("class"))) (attrs)>
            // Keys keep each code block's copy button state as the text grows.
            #[key(index)]
            for (index, block) in blocks.into_iter().enumerate() {
                match block {
                    Block::Html(html) => (Unescaped::new_unchecked(html)),
                    Block::Code { language, code } => {
                        code_block(language: language.as_deref(), code: &code)
                    }
                }
            }
        </div>
    })
}

/// A run of rendered markdown: HTML for everything except fenced and indented code,
/// which renders through [`code_block`].
///
/// A code block inside a list item or quote splits the surrounding HTML, so a run may
/// open elements that a later run closes. The runs are always rendered in order, so the
/// document as a whole is balanced.
#[derive(Debug, PartialEq)]
enum Block {
    /// Markup produced by [`render`] from escaped text and allowed tags only.
    Html(String),
    /// A code block.
    Code {
        language: Option<String>,
        code: String,
    },
}

/// URL schemes links may use.
const LINK_SCHEMES: &[&str] = &["http", "https", "mailto"];

/// URL schemes images may load from.
const IMAGE_SCHEMES: &[&str] = &["http", "https"];

/// Parses `source` into safe HTML runs and code blocks.
///
/// Raw HTML events become text, which the HTML writer escapes. Every tag left as
/// [`Event::Html`] afterwards is generated here from escaped values.
fn render(source: &str) -> Vec<Block> {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut parser = Parser::new_ext(source, options);

    let mut blocks = Vec::new();
    let mut events: Vec<Event<'_>> = Vec::new();
    // Whether each open link rendered an `<a>`, so its end tag matches.
    let mut links: Vec<bool> = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => info
                        .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
                        .next()
                        .filter(|language| !language.is_empty())
                        .map(str::to_owned),
                    CodeBlockKind::Indented => None,
                };
                let mut code = String::new();
                for event in parser.by_ref() {
                    match event {
                        Event::Text(text) => code.push_str(&text),
                        Event::End(TagEnd::CodeBlock) => break,
                        _ => {}
                    }
                }
                // The fence's closing line break is part of the text.
                if code.ends_with('\n') {
                    code.pop();
                }
                flush(&mut blocks, &mut events);
                blocks.push(Block::Code { language, code });
            }
            // Raw HTML is shown as text. A block of it becomes a paragraph.
            Event::Start(Tag::HtmlBlock) => events.push(Event::Start(Tag::Paragraph)),
            Event::End(TagEnd::HtmlBlock) => events.push(Event::End(TagEnd::Paragraph)),
            Event::Html(raw) | Event::InlineHtml(raw) => events.push(Event::Text(raw)),
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => match allowed_url(&dest_url, LINK_SCHEMES) {
                Some(href) => {
                    let mut tag = format!(
                        r#"<a href="{}" rel="noopener noreferrer" target="_blank""#,
                        escape(href)
                    );
                    if !title.is_empty() {
                        tag.push_str(&format!(r#" title="{}""#, escape(&title)));
                    }
                    tag.push('>');
                    events.push(Event::Html(tag.into()));
                    links.push(true);
                }
                None => links.push(false),
            },
            Event::End(TagEnd::Link) => {
                if links.pop() == Some(true) {
                    events.push(Event::Html("</a>".into()));
                }
            }
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                let alt = image_alt(&mut parser);
                match allowed_url(&dest_url, IMAGE_SCHEMES) {
                    Some(src) => {
                        let mut tag = format!(
                            r#"<img src="{}" alt="{}" loading="lazy" referrerpolicy="no-referrer""#,
                            escape(src),
                            escape(&alt)
                        );
                        if !title.is_empty() {
                            tag.push_str(&format!(r#" title="{}""#, escape(&title)));
                        }
                        tag.push('>');
                        events.push(Event::Html(tag.into()));
                    }
                    None => events.push(Event::Text(alt.into())),
                }
            }
            Event::Start(Tag::Table(alignments)) => {
                events.push(Event::Html(TABLE_WRAPPER.into()));
                events.push(Event::Start(Tag::Table(alignments)));
            }
            Event::End(TagEnd::Table) => {
                events.push(Event::End(TagEnd::Table));
                events.push(Event::Html("</div>".into()));
            }
            event => events.push(event),
        }
    }
    flush(&mut blocks, &mut events);
    blocks
}

/// Writes the pending events as an HTML run.
fn flush(blocks: &mut Vec<Block>, events: &mut Vec<Event<'_>>) {
    if events.is_empty() {
        return;
    }
    let mut out = String::new();
    html::push_html(&mut out, events.drain(..));
    blocks.push(Block::Html(out));
}

/// Consumes an image's inner events up to its end and returns their text, which is the
/// image's alternative text.
fn image_alt(parser: &mut Parser<'_>) -> String {
    let mut alt = String::new();
    let mut depth = 0usize;
    for event in parser.by_ref() {
        match event {
            Event::Start(Tag::Image { .. }) => depth += 1,
            Event::End(TagEnd::Image) if depth == 0 => break,
            Event::End(TagEnd::Image) => depth -= 1,
            Event::Text(text) | Event::Code(text) => alt.push_str(&text),
            _ => {}
        }
    }
    alt
}

/// Returns `url` if it has one of `schemes`, compared without regard to case.
///
/// Relative URLs and URLs containing control characters or whitespace are rejected:
/// browsers strip tabs and line breaks inside URLs and spaces around them, which could
/// otherwise turn a harmless-looking scheme into `javascript:`.
fn allowed_url<'u>(url: &'u str, schemes: &[&str]) -> Option<&'u str> {
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return None;
    }
    let (scheme, _) = url.split_once(':')?;
    schemes
        .iter()
        .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
        .then_some(url)
}

/// Escapes text for an HTML attribute value in double quotes.
fn escape(value: &str) -> CowStr<'static> {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out.into()
}

#[cfg(test)]
mod tests {
    use super::{Block, render};

    /// Renders `source` and joins its HTML runs, marking code blocks.
    fn html(source: &str) -> String {
        render(source)
            .into_iter()
            .map(|block| match block {
                Block::Html(html) => html,
                Block::Code { language, code } => {
                    format!("[code {}: {code}]", language.unwrap_or_default())
                }
            })
            .collect()
    }

    #[test]
    fn renders_basic_markdown() {
        let out = html("# Title\n\nSome *emphasis* and `code`.\n\n- one\n- two\n");
        assert!(out.contains("<h1>Title</h1>"), "{out}");
        assert!(out.contains("<em>emphasis</em>"), "{out}");
        assert!(out.contains("<code>code</code>"), "{out}");
        assert!(out.contains("<li>one</li>"), "{out}");
    }

    #[test]
    fn escapes_raw_html_blocks() {
        let out = html("<script>alert(1)</script>\n\n<div onclick=\"x()\">hi</div>\n");
        assert!(!out.contains("<script"), "{out}");
        assert!(!out.contains("<div"), "{out}");
        assert!(
            out.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
            "{out}"
        );
    }

    #[test]
    fn escapes_inline_html() {
        let out = html("Hello <img src=x onerror=alert(1)> and <b>bold</b>.");
        assert!(!out.contains("<img"), "{out}");
        assert!(!out.contains("<b>"), "{out}");
        assert!(out.contains("&lt;img src=x onerror=alert(1)&gt;"), "{out}");
    }

    #[test]
    fn drops_javascript_links() {
        for source in [
            "[click](javascript:alert(1))",
            "[click](JavaScript:alert(1))",
            "[click](<java\tscript:alert(1)>)",
            "[click](data:text/html,<script>alert(1)</script>)",
            "[click](vbscript:msgbox)",
            "<javascript:alert(1)>",
            "[click][ref]\n\n[ref]: javascript:alert(1)",
        ] {
            let out = html(source);
            assert!(!out.contains("<a"), "{source:?} rendered {out}");
            assert!(
                !out.to_lowercase().contains("href"),
                "{source:?} rendered {out}"
            );
        }
        assert!(html("[click](javascript:alert(1))").contains("click"));
    }

    #[test]
    fn keeps_safe_links_and_opens_them_in_a_new_tab() {
        let out = html("[docs](https://example.com/a?b=1&c=\"2\") and <mailto:me@example.com>");
        assert!(
            out.contains(concat!(
                r#"<a href="https://example.com/a?b=1&amp;c=&quot;2&quot;" "#,
                r#"rel="noopener noreferrer" target="_blank">docs</a>"#,
            )),
            "{out}"
        );
        assert!(out.contains(r#"href="mailto:me@example.com""#), "{out}");
    }

    #[test]
    fn drops_relative_links() {
        let out = html("[home](/home) and [frag](#top)");
        assert!(!out.contains("<a"), "{out}");
    }

    #[test]
    fn escapes_link_titles_and_image_attributes() {
        let out = html(
            r#"[x](https://example.com "a\"onmouseover=\"alert(1)") ![a"b](https://example.com/i.png)"#,
        );
        assert!(!out.contains(r#""onmouseover"#), "{out}");
        assert!(out.contains(r#"alt="a&quot;b""#), "{out}");
    }

    #[test]
    fn renders_unsafe_images_as_text() {
        let out = html("![tracker](javascript:alert(1)) ![pic](data:image/png;base64,AAAA)");
        assert!(!out.contains("<img"), "{out}");
        assert!(out.contains("tracker"), "{out}");
    }

    #[test]
    fn renders_code_blocks_separately() {
        let blocks = render("Before\n\n```rust title=\"x\"\nfn main() {}\n```\n\nAfter");
        assert_eq!(blocks.len(), 3);
        assert_eq!(
            blocks[1],
            Block::Code {
                language: Some("rust".to_owned()),
                code: "fn main() {}".to_owned(),
            }
        );
    }

    #[test]
    fn code_block_contents_stay_raw_for_escaping_by_the_view() {
        let blocks = render("```html\n<script>alert(1)</script>\n```");
        assert_eq!(
            blocks,
            vec![Block::Code {
                language: Some("html".to_owned()),
                code: "<script>alert(1)</script>".to_owned(),
            }]
        );
    }

    #[test]
    fn renders_an_unclosed_code_fence() {
        let blocks = render("Here you go:\n\n```python\ndef f():\n    return 1");
        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[1],
            Block::Code {
                language: Some("python".to_owned()),
                code: "def f():\n    return 1".to_owned(),
            }
        );
    }

    #[test]
    fn tolerates_incomplete_markdown() {
        for source in [
            "[half a li",
            "[half a link](https://exa",
            "**bold",
            "| a | b |\n| -",
        ] {
            let out = html(source);
            assert!(!out.contains("<a"), "{source:?} rendered {out}");
        }
    }

    #[test]
    fn code_block_inside_a_list_keeps_the_document_balanced() {
        let out = html("- item\n\n  ```\n  code\n  ```\n- next\n");
        assert_eq!(
            out.matches("<li>").count(),
            out.matches("</li>").count(),
            "{out}"
        );
        assert_eq!(
            out.matches("<ul>").count(),
            out.matches("</ul>").count(),
            "{out}"
        );
    }

    #[test]
    fn renders_gfm_extensions() {
        let out = html("~~gone~~\n\n- [x] done\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(out.contains("<del>gone</del>"), "{out}");
        assert!(out.contains(r#"type="checkbox""#), "{out}");
        assert!(out.contains("<table>"), "{out}");
        assert!(out.contains(r#"data-slot="markdown-table""#), "{out}");
    }
}
