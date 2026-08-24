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
        let candidate = &template[value_start..close];

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
        let candidate = &template[value_start..close];

        if let Some(name) = candidate.strip_prefix("prompt:")
            && super::validate_name(name).is_ok()
        {
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
        let template = "Language: {{language}}\n\n{{input}}";
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
    fn leaves_non_template_braces_unchanged() {
        let template = "{{ invalid }} and {value} and {{1bad}}";
        assert_eq!(render(template, &HashMap::new()).unwrap(), template);
    }

    #[test]
    fn supports_repeated_variables() {
        assert_eq!(
            render("{{x}}/{{x}}", &values(&[("x", "a")])).unwrap(),
            "a/a"
        );
    }

    #[test]
    fn finds_valid_nested_opening_after_invalid_candidate() {
        let template = "{{ invalid {{value}}";
        assert_eq!(
            render(template, &values(&[("value", "ok")])).unwrap(),
            "{{ invalid ok"
        );
    }

    #[test]
    fn detects_prompt_composition_references_separately() {
        assert_eq!(
            compositions("before {{prompt:shared.rules}} after"),
            vec![Composition {
                start: 7,
                end: 30,
                name: "shared.rules".into(),
            }]
        );
        assert!(placeholders("{{prompt:shared.rules}}").is_empty());
        assert!(compositions("{{prompt:invalid name}}").is_empty());
    }
}
