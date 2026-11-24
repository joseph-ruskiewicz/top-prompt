//! Reads the API tables of the docs from the component sources themselves.
//!
//! Every public `#[component]` function becomes a table with a row per prop: its name,
//! type, default and the first sentence of its doc comment. Reading the sources keeps
//! the tables from drifting: changing a prop changes its row. The sources are
//! rustfmt-formatted, so a line-based reader is enough.

/// One public component function.
#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub props: Vec<Prop>,
}

/// One prop of a component function.
#[derive(Debug, PartialEq, Eq)]
pub struct Prop {
    /// The name as written in `view!`; the child content is `children`.
    pub name: String,
    pub ty: String,
    /// The default as Rust source, or `None` when the prop is required.
    pub default: Option<String>,
    pub description: String,
}

/// The component functions of `source`, in source order.
pub fn functions(source: &str) -> Vec<Function> {
    let lines: Vec<&str> = source.lines().collect();
    let mut functions = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].trim() != "#[component]" {
            i += 1;
            continue;
        }
        let Some(name) = lines
            .get(i + 1)
            .and_then(|line| line.trim().strip_prefix("pub async fn "))
            .and_then(|rest| rest.strip_suffix('('))
        else {
            i += 1;
            continue;
        };

        let mut props = Vec::new();
        let mut docs = Vec::new();
        let mut default = None;
        i += 2;
        while i < lines.len() && !lines[i].trim_start().starts_with(')') {
            let line = lines[i].trim();
            if let Some(doc) = line.strip_prefix("///") {
                docs.push(doc.trim());
            } else if line == "#[default]" {
                default = Some(String::new());
            } else if let Some(value) = line
                .strip_prefix("#[default(")
                .and_then(|rest| rest.strip_suffix(")]"))
            {
                default = Some(value.to_owned());
            } else if line.starts_with("#[") {
                // `#[into]` and other attributes don't change the table.
            } else if let Some((param, ty)) = line.trim_end_matches(',').split_once(": ") {
                let param = param.trim_start_matches("mut ");
                if param != "cx" {
                    let default = default.take().map(|value| {
                        if value.is_empty() {
                            implied_default(ty, source)
                        } else {
                            simplify(&value)
                        }
                    });
                    props.push(Prop {
                        name: if param == "child" { "children" } else { param }.to_owned(),
                        ty: ty.to_owned(),
                        default,
                        description: first_sentence(&docs.join(" ")),
                    });
                }
                docs.clear();
                default = None;
            }
            i += 1;
        }
        functions.push(Function {
            name: name.to_owned(),
            props,
        });
    }

    functions
}

/// What `#[default]` gives a prop of type `ty`, as Rust source.
fn implied_default(ty: &str, source: &str) -> String {
    match ty {
        "bool" => "false".to_owned(),
        "u32" | "usize" | "u64" | "i32" => "0".to_owned(),
        "&str" | "String" => "\"\"".to_owned(),
        _ if ty.starts_with("Option<") => "None".to_owned(),
        _ if ty == "Attributes" || ty.starts_with("Child") => String::new(),
        _ => enum_default(ty, source).unwrap_or_else(|| format!("{ty}::default()")),
    }
}

/// The variant of the enum `ty` in `source` marked `#[default]`.
fn enum_default(ty: &str, source: &str) -> Option<String> {
    let start = source.find(&format!("pub enum {ty} {{"))?;
    let body = &source[start..];
    let body = &body[..body.find("\n}")?];
    let mut lines = body.lines().map(str::trim);
    lines.find(|line| *line == "#[default]")?;
    let variant = lines.find(|line| !line.starts_with("///") && !line.starts_with("#["))?;
    Some(format!("{ty}::{}", variant.trim_end_matches(',')))
}

/// Shortens default expressions to the value a reader cares about.
fn simplify(value: &str) -> String {
    for (prefix, suffix) in [("String::from(", ")"), ("Expr::from(", ")")] {
        if let Some(inner) = value
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix(suffix))
        {
            return inner.to_owned();
        }
    }
    value.to_owned()
}

/// The first sentence of a doc comment, without its Markdown link targets.
fn first_sentence(docs: &str) -> String {
    let end = docs
        .match_indices(". ")
        .map(|(index, _)| index + 1)
        .next()
        .unwrap_or(docs.len());
    docs[..end].replace("[`", "`").replace("`]", "`")
}

#[cfg(test)]
mod tests {
    use super::{Function, Prop, functions};

    const SOURCE: &str = r#"
/// The animation.
#[derive(Default)]
pub enum Variant {
    /// Dots.
    #[default]
    Dots,
    Pulse,
}

/// A thing.
#[component]
pub async fn thing(
    cx: &Cx,
    /// The animation to show. More text.
    #[default]
    variant: Variant,
    /// The label, for [`thing`].
    #[into]
    #[default(String::from("Thinking…"))]
    label: String,
    /// The URL.
    href: &str,
    /// Extra attributes.
    #[default]
    mut attrs: Attributes,
    /// The content.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    todo!()
}

async fn private_helper(cx: &Cx) {}
"#;

    #[test]
    fn reads_props_types_defaults_and_docs() {
        let prop = |name: &str, ty: &str, default: Option<&str>, description: &str| Prop {
            name: name.to_owned(),
            ty: ty.to_owned(),
            default: default.map(str::to_owned),
            description: description.to_owned(),
        };
        assert_eq!(
            functions(SOURCE),
            [Function {
                name: "thing".to_owned(),
                props: vec![
                    prop(
                        "variant",
                        "Variant",
                        Some("Variant::Dots"),
                        "The animation to show."
                    ),
                    prop(
                        "label",
                        "String",
                        Some("\"Thinking…\""),
                        "The label, for `thing`."
                    ),
                    prop("href", "&str", None, "The URL."),
                    prop("attrs", "Attributes", Some(""), "Extra attributes."),
                    prop("children", "Child<'_>", Some(""), "The content."),
                ],
            }]
        );
    }

    #[test]
    fn every_registry_component_documents_every_prop() {
        for (name, source) in crate::catalog::registry_sources() {
            let functions = functions(source);
            assert!(!functions.is_empty(), "{name} has no component functions");
            for function in functions {
                for prop in function.props {
                    assert!(
                        !prop.description.is_empty(),
                        "{name}::{} prop `{}` has no doc comment",
                        function.name,
                        prop.name,
                    );
                }
            }
        }
    }
}
