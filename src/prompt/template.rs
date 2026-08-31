use std::collections::HashMap;

use crate::error::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Placeholder {
    pub start: usize,
    pub end: usize,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Composition {
    pub start: usize,
    pub end: usize,
    pub name: String,
}

pub fn is_valid_variable_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

pub fn validate(template: &str) -> Result<()> {
    validate_with_line_offset(template, 0)
}

pub(crate) fn validate_with_line_offset(template: &str, line_offset: usize) -> Result<()> {
    let mut cursor = 0;

    loop {
        let opening = template[cursor..]
            .find("{{")
            .map(|position| cursor + position);

        let Some(opening) = opening else {
            return Ok(());
        };
        let value_start = opening + 2;
        let Some(relative_closing) = template[value_start..].find("}}") else {
            return Err(syntax_error(
                template,
                opening,
                line_offset,
                "unclosed `{{`",
            ));
        };
        let closing = value_start + relative_closing;
        let candidate = template[value_start..closing].trim();

        if !is_valid_variable_name(candidate) && composition_name(candidate).is_none() {
            return Err(syntax_error(
                template,
                opening,
                line_offset,
                &format!("invalid expression `{}`", &template[opening..closing + 2]),
            ));
        }

        cursor = closing + 2;
    }
}

pub fn placeholders(template: &str) -> Vec<Placeholder> {
    let bytes = template.as_bytes();
    let mut result = Vec::new();
    let mut cursor = 0;

    while let Some(relative_open) = find_pair(&bytes[cursor..], b'{', b'{') {
        let open = cursor + relative_open;
        let value_start = open + 2;
        let Some(relative_close) = find_pair(&bytes[value_start..], b'}', b'}') else {
            break;
        };
        let close = value_start + relative_close;
        let candidate = template[value_start..close].trim();

        if is_valid_variable_name(candidate) {
            result.push(Placeholder {
                start: open,
                end: close + 2,
                name: candidate.to_owned(),
            });
            cursor = close + 2;
        } else {
            cursor = value_start;
        }
    }

    result
}

pub fn compositions(template: &str) -> Vec<Composition> {
    let bytes = template.as_bytes();
    let mut result = Vec::new();
    let mut cursor = 0;

    while let Some(relative_open) = find_pair(&bytes[cursor..], b'{', b'{') {
        let open = cursor + relative_open;
        let value_start = open + 2;
        let Some(relative_close) = find_pair(&bytes[value_start..], b'}', b'}') else {
            break;
        };
        let close = value_start + relative_close;
        let candidate = template[value_start..close].trim();

        if let Some(name) = composition_name(candidate) {
            result.push(Composition {
                start: open,
                end: close + 2,
                name: name.to_owned(),
            });
            cursor = close + 2;
        } else {
            cursor = value_start;
        }
    }

    result
}

pub fn render(template: &str, values: &HashMap<String, String>) -> Result<String> {
    validate(template)?;
    let placeholders = placeholders(template);
    for placeholder in &placeholders {
        if !values.contains_key(&placeholder.name) {
            return Err(Error::MissingVariable(placeholder.name.clone()));
        }
    }

    let extra_capacity: usize = placeholders
        .iter()
        .filter_map(|placeholder| values.get(&placeholder.name))
        .map(String::len)
        .sum();
    let mut output = String::with_capacity(template.len() + extra_capacity);
    let mut cursor = 0;
    for placeholder in placeholders {
        output.push_str(&template[cursor..placeholder.start]);
        output.push_str(&values[&placeholder.name]);
        cursor = placeholder.end;
    }
    output.push_str(&template[cursor..]);
    Ok(output)
}

fn composition_name(candidate: &str) -> Option<&str> {
    candidate
        .strip_prefix("prompt:")
        .filter(|name| super::validate_name(name).is_ok())
}

fn syntax_error(template: &str, position: usize, line_offset: usize, message: &str) -> Error {
    let prefix = &template[..position];
    let line = line_offset + prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix, |(_, line)| line)
        .chars()
        .count()
        + 1;
    Error::Message(format!(
        "invalid template syntax at line {line}, column {column}: {message}"
    ))
}

fn find_pair(bytes: &[u8], first: u8, second: u8) -> Option<usize> {
    bytes
        .windows(2)
        .position(|window| window[0] == first && window[1] == second)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(key, value)| ((*key).into(), (*value).into()))
            .collect()
    }

    #[test]
    fn detects_and_renders_variables() {
        let template = "Language: {{ language }}\n\n{{input}}";
        assert_eq!(
            render(
                template,
                &values(&[("language", "rust"), ("input", "fn main() {}")])
            )
            .unwrap(),
            "Language: rust\n\nfn main() {}"
        );
    }

    #[test]
    fn reports_the_first_missing_variable() {
        let error = render("{{first}} {{second}}", &HashMap::new()).unwrap_err();
        assert!(matches!(error, Error::MissingVariable(name) if name == "first"));
    }

    #[test]
    fn rejects_invalid_template_expressions() {
        for template in [
            "{{ daily report content }}",
            "{{1bad}}",
            "{{prompt:invalid name}}",
            "{{value",
        ] {
            assert!(validate(template).is_err(), "{template} should be invalid");
        }
        assert_eq!(
            validate("line one\n{{ invalid name }}")
                .unwrap_err()
                .to_string(),
            "invalid template syntax at line 2, column 1: invalid expression `{{ invalid name }}`"
        );
    }

    #[test]
    fn allows_whitespace_around_expression_names() {
        validate("{{ daily_report_content }} {{ prompt:shared.rules }}").unwrap();
        assert_eq!(
            render(
                "Report: {{ daily_report_content }}",
                &values(&[("daily_report_content", "done")])
            )
            .unwrap(),
            "Report: done"
        );
    }

    #[test]
    fn supports_repeated_variables() {
        assert_eq!(
            render("{{x}}/{{x}}", &values(&[("x", "a")])).unwrap(),
            "a/a"
        );
    }

    #[test]
    fn leaves_single_braces_unchanged() {
        let template = "{value} and JSON: {\"outer\": {\"key\": true}}";
        assert_eq!(render(template, &HashMap::new()).unwrap(), template);
    }

    #[test]
    fn detects_prompt_composition_references_separately() {
        assert_eq!(
            compositions("before {{ prompt:shared.rules }} after"),
            vec![Composition {
                start: 7,
                end: 32,
                name: "shared.rules".into(),
            }]
        );
        assert!(placeholders("{{prompt:shared.rules}}").is_empty());
        assert!(compositions("{{prompt:invalid name}}").is_empty());
    }
}
