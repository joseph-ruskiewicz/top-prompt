//! The tools the model can call. Both are cheap and deterministic, read nothing but
//! their input and the docs catalog compiled into the binary, and make no network
//! calls.

use serde_json::{Value, json};

use crate::catalog;

/// The tool definitions sent with every request.
pub fn definitions() -> Value {
    json!([
        {
            "name": "search_components",
            "description": "Search the topcoat-prompt component catalog by keyword. Returns \
                up to five matching components with their name, title, description and docs \
                path. Use it to answer questions about which components exist or what one \
                does.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Words to look for, such as \"tool call\" or \"streaming\"."
                    }
                },
                "required": ["query"]
            },
            // Streams the input as it is generated; the input is validated below
            // before the tool runs.
            "eager_input_streaming": true
        },
        {
            "name": "calculate",
            "description": "Evaluate an arithmetic expression exactly. Supports + - * / % ^, \
                parentheses and decimal numbers. Returns the result as a number.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "The expression, such as \"(17.5 * 4) / 3\"."
                    }
                },
                "required": ["expression"]
            },
            "eager_input_streaming": true
        }
    ])
}

/// The longest string input either tool accepts, in bytes.
const MAX_INPUT: usize = 200;

/// Runs the tool `name` on `input`. `Err` is a message for an `is_error` result,
/// written so the model can correct its call.
pub fn run(name: &str, input: &Value) -> Result<String, String> {
    match name {
        "search_components" => search(string_input(input, "query")?),
        "calculate" => calculate(string_input(input, "expression")?),
        _ => Err(format!("There is no tool named {name}.")),
    }
}

/// The string field `field` of an input object with no other fields, checked
/// against the schema by hand.
fn string_input<'a>(input: &'a Value, field: &str) -> Result<&'a str, String> {
    let object = input
        .as_object()
        .ok_or_else(|| "The input must be a JSON object.".to_owned())?;
    if let Some(extra) = object.keys().find(|name| *name != field) {
        return Err(format!(
            "Unexpected field `{extra}`; the only field is `{field}`."
        ));
    }
    let value = object
        .get(field)
        .ok_or_else(|| format!("Missing required field `{field}`."))?
        .as_str()
        .ok_or_else(|| format!("`{field}` must be a string."))?;
    if value.trim().is_empty() {
        return Err(format!("`{field}` is empty."));
    }
    if value.len() > MAX_INPUT {
        return Err(format!("`{field}` is longer than {MAX_INPUT} bytes."));
    }
    Ok(value)
}

/// Ranks the catalog's components by how many query words their name, title and
/// description contain.
fn search(query: &str) -> Result<String, String> {
    let words: Vec<String> = query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.len() > 1)
        .map(str::to_owned)
        .collect();
    let mut hits: Vec<(usize, &catalog::ComponentDoc)> = catalog::COMPONENTS
        .iter()
        .map(|doc| {
            let name = doc.name.replace('_', " ");
            let title = doc.title.to_lowercase();
            let description = doc.description.to_lowercase();
            let score = words
                .iter()
                .map(|word| {
                    3 * usize::from(name.contains(word.as_str()))
                        + 2 * usize::from(title.contains(word.as_str()))
                        + usize::from(description.contains(word.as_str()))
                })
                .sum();
            (score, doc)
        })
        .filter(|(score, _)| *score > 0)
        .collect();
    // Highest score first; the catalog order breaks ties, so results are stable.
    hits.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    let results: Vec<Value> = hits
        .into_iter()
        .take(5)
        .map(|(_, doc)| {
            json!({
                "name": doc.name,
                "title": doc.title,
                "description": doc.description,
                "docs": format!("/docs/{}", doc.name),
            })
        })
        .collect();
    Ok(serde_json::to_string_pretty(&json!({ "results": results })).unwrap_or_default())
}

/// Evaluates an arithmetic expression.
fn calculate(expression: &str) -> Result<String, String> {
    let mut parser = Calc {
        chars: expression.chars().filter(|c| !c.is_whitespace()).collect(),
        at: 0,
        depth: 0,
    };
    let value = parser.sum()?;
    if parser.at != parser.chars.len() {
        return Err(format!(
            "Unexpected `{}` at position {}.",
            parser.chars[parser.at],
            parser.at + 1
        ));
    }
    if !value.is_finite() {
        return Err("The result is not a finite number.".to_owned());
    }
    // Round to 15 significant digits, which f64 holds exactly, then print the
    // shortest form: 0.1 + 0.2 is 0.3, not 0.30000000000000004.
    let rounded: f64 = format!("{value:.14e}")
        .parse()
        .map_err(|_| "The result is not a finite number.".to_owned())?;
    Ok(format!("{rounded}"))
}

/// A recursive-descent parser: sum → product → unary → power → atom.
struct Calc {
    chars: Vec<char>,
    at: usize,
    depth: usize,
}

impl Calc {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn sum(&mut self) -> Result<f64, String> {
        let mut value = self.product()?;
        while let Some(op @ ('+' | '-')) = self.peek() {
            self.at += 1;
            let right = self.product()?;
            value = if op == '+' {
                value + right
            } else {
                value - right
            };
        }
        Ok(value)
    }

    fn product(&mut self) -> Result<f64, String> {
        let mut value = self.unary()?;
        while let Some(op @ ('*' | '/' | '%')) = self.peek() {
            self.at += 1;
            let right = self.unary()?;
            if op != '*' && right == 0.0 {
                return Err("Division by zero.".to_owned());
            }
            value = match op {
                '*' => value * right,
                '/' => value / right,
                _ => value % right,
            };
        }
        Ok(value)
    }

    /// A sign applies after powers, so -2^2 is -4.
    fn unary(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some('-') => {
                self.at += 1;
                Ok(-self.unary()?)
            }
            Some('+') => {
                self.at += 1;
                self.unary()
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<f64, String> {
        let base = self.atom()?;
        if self.peek() == Some('^') {
            self.at += 1;
            // Right-associative, and the exponent may have a sign: 2^3^2 is 2^9.
            let exponent = self.unary()?;
            return Ok(base.powf(exponent));
        }
        Ok(base)
    }

    fn atom(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some('(') => {
                self.depth += 1;
                if self.depth > 32 {
                    return Err("Too many nested parentheses.".to_owned());
                }
                self.at += 1;
                let value = self.sum()?;
                if self.peek() != Some(')') {
                    return Err("Missing a closing parenthesis.".to_owned());
                }
                self.at += 1;
                self.depth -= 1;
                Ok(value)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => {
                let start = self.at;
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
                    self.at += 1;
                }
                let number: String = self.chars[start..self.at].iter().collect();
                number
                    .parse()
                    .map_err(|_| format!("`{number}` is not a number."))
            }
            Some(c) => Err(format!(
                "Unexpected `{c}` at position {}. Use numbers, + - * / % ^ and parentheses.",
                self.at + 1
            )),
            None => Err("The expression ended early.".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calc(expression: &str) -> Result<String, String> {
        run("calculate", &json!({ "expression": expression }))
    }

    #[test]
    fn calculates() {
        assert_eq!(calc("6 * 7").unwrap(), "42");
        assert_eq!(calc("(17.5 * 4) / 3").unwrap(), "23.3333333333333");
        assert_eq!(calc("1234 * 5678 / 9").unwrap(), "778516.888888889");
        assert_eq!(calc("2^60").unwrap(), "1152921504606850000");
        assert_eq!(calc("2^3^2").unwrap(), "512");
        assert_eq!(calc("-2^2").unwrap(), "-4");
        assert_eq!(calc("10 % 4 - -1").unwrap(), "3");
        assert_eq!(calc("2^-1").unwrap(), "0.5");
        assert_eq!(calc("0.1 + 0.2").unwrap(), "0.3");
    }

    #[test]
    fn rejects_bad_expressions() {
        assert!(calc("1 / 0").unwrap_err().contains("zero"));
        assert!(calc("2 +").is_err());
        assert!(calc("(1 + 2").unwrap_err().contains("parenthesis"));
        assert!(calc("1 + x").unwrap_err().contains("`x`"));
        assert!(calc("1.2.3").is_err());
        assert!(calc(&"(".repeat(40)).is_err());
        assert!(calc("10^400").unwrap_err().contains("finite"));
        assert!(calc(&"1+".repeat(150)).unwrap_err().contains("longer"));
    }

    #[test]
    fn validates_input_against_the_schema() {
        assert!(run("calculate", &json!("6*7")).is_err());
        assert!(
            run("calculate", &json!({}))
                .unwrap_err()
                .contains("Missing")
        );
        assert!(run("calculate", &json!({ "expression": 42 })).is_err());
        assert!(
            run("calculate", &json!({ "expression": "1", "extra": true }))
                .unwrap_err()
                .contains("extra")
        );
        assert!(run("fetch_url", &json!({})).is_err());
    }

    #[test]
    fn searches_the_catalog() {
        let result = run("search_components", &json!({ "query": "tool call" })).unwrap();
        let result: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(result["results"][0]["name"], "tool_call");
        assert_eq!(result["results"][0]["docs"], "/docs/tool_call");

        let none = run("search_components", &json!({ "query": "zzzz" })).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&none).unwrap()["results"],
            json!([])
        );
    }

    #[test]
    fn same_input_same_output() {
        let input = json!({ "query": "message" });
        assert_eq!(
            run("search_components", &input),
            run("search_components", &input)
        );
    }
}
