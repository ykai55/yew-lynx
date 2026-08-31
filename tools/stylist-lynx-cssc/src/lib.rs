use std::collections::BTreeMap;

use serde::Serialize;
use stylist_core::ast::{Block, RuleBlockContent, ScopeContent, Sheet, StringFragment};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleRule {
    #[serde(rename = "type")]
    rule_type: &'static str,
    selector_text: SelectorText,
    style: Vec<Declaration>,
    variables: BTreeMap<String, String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct SelectorText {
    value: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Declaration {
    name: String,
    value: String,
}

pub fn convert(css: &str, class: &str) -> Result<Vec<StyleRule>, String> {
    validate_class(class)?;

    if css.contains("${") {
        return Err("Stylist interpolation is not supported".into());
    }

    // Stylist's parser does not expose unconsumed input, so a final AST marker
    // makes a successful parse of the complete input observable.
    let css_with_marker =
        format!("{css}\n#__stylist_lynx_cssc_end__ {{ --stylist-lynx-cssc-end: 1; }}\n");
    let sheet: Sheet = css_with_marker
        .parse()
        .map_err(|error| format!("failed to parse Stylist CSS: {error}"))?;
    let Some((marker, scopes)) = sheet.split_last() else {
        return Err("failed to parse the complete Stylist CSS input".into());
    };
    if !is_end_marker(marker) {
        return Err("failed to parse the complete Stylist CSS input".into());
    }
    let mut rules = Vec::with_capacity(scopes.len());

    for scope in scopes {
        let ScopeContent::Block(block) = scope else {
            return Err("at-rules are not supported".into());
        };
        rules.push(convert_block(block, class)?);
    }

    if rules.is_empty() {
        return Err("the stylesheet must produce at least one rule".into());
    }

    Ok(rules)
}

fn is_end_marker(scope: &ScopeContent) -> bool {
    let ScopeContent::Block(block) = scope else {
        return false;
    };
    let [selector] = block.condition.as_ref() else {
        return false;
    };
    let [selector_fragment] = selector.fragments.as_ref() else {
        return false;
    };
    let [RuleBlockContent::StyleAttr(attribute)] = block.content.as_ref() else {
        return false;
    };
    let [value_fragment] = attribute.value.as_ref() else {
        return false;
    };

    selector_fragment.inner == "#__stylist_lynx_cssc_end__"
        && attribute.key == "--stylist-lynx-cssc-end"
        && value_fragment.inner == "1"
}

fn validate_class(class: &str) -> Result<(), String> {
    let mut bytes = class.bytes();
    let Some(first) = bytes.next() else {
        return Err("class must not be empty".into());
    };
    let second = bytes.clone().next();
    if !(first.is_ascii_alphabetic() || first == b'-' || first == b'_')
        || (first == b'-' && (second.is_none() || second.is_some_and(|byte| byte.is_ascii_digit())))
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(
            "class must use ASCII letters, digits, '-' or '_', start like a CSS identifier, and not be '-' alone"
                .into(),
        );
    }
    Ok(())
}

fn convert_block(block: &Block, class: &str) -> Result<StyleRule, String> {
    let selector = if block.condition.is_empty() {
        format!(".{class}")
    } else {
        block
            .condition
            .iter()
            .map(|selector| {
                let condition = join_fragments(&selector.fragments)?;
                if condition.is_empty() {
                    return Err("selector must not be empty".into());
                }

                let scoped_class = format!(".{class}");
                let (condition, replaced_scope) = replace_scope_tokens(&condition, &scoped_class);
                if replaced_scope {
                    Ok(condition)
                } else if condition.starts_with(':') {
                    Ok(format!("{scoped_class}{condition}"))
                } else {
                    Ok(format!("{scoped_class} {condition}"))
                }
            })
            .collect::<Result<Vec<_>, String>>()?
            .join(", ")
    };

    let mut declarations = Vec::with_capacity(block.content.len());
    for content in block.content.iter() {
        let RuleBlockContent::StyleAttr(attribute) = content else {
            return Err("nested blocks and at-rules are not supported".into());
        };
        let name = attribute.key.to_string();
        let value = join_fragments(&attribute.value)?;
        if name.is_empty() || value.is_empty() {
            return Err("declaration names and values must not be empty".into());
        }
        declarations.push(Declaration { name, value });
    }

    if declarations.is_empty() {
        return Err("each rule must contain at least one declaration".into());
    }

    Ok(StyleRule {
        rule_type: "StyleRule",
        selector_text: SelectorText { value: selector },
        style: declarations,
        variables: BTreeMap::new(),
    })
}

fn replace_scope_tokens(selector: &str, scoped_class: &str) -> (String, bool) {
    let bytes = selector.as_bytes();
    let mut output = String::with_capacity(selector.len());
    let mut copied_until = 0;
    let mut bracket_depth: usize = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut replaced = false;
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if let Some(active_quote) = quote {
            if byte == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
            index += 1;
            continue;
        }
        if byte == b'[' {
            bracket_depth += 1;
            index += 1;
            continue;
        }
        if byte == b']' {
            bracket_depth = bracket_depth.saturating_sub(1);
            index += 1;
            continue;
        }
        if bracket_depth != 0 {
            index += 1;
            continue;
        }

        let token_len = if byte == b'&' {
            1
        } else if byte == b':'
            && selector[index..].starts_with(":root")
            && bytes
                .get(index + 5)
                .is_none_or(|next| !next.is_ascii_alphanumeric() && *next != b'-' && *next != b'_')
        {
            5
        } else {
            index += 1;
            continue;
        };
        output.push_str(&selector[copied_until..index]);
        output.push_str(scoped_class);
        index += token_len;
        copied_until = index;
        replaced = true;
    }

    if !replaced {
        return (selector.to_owned(), false);
    }
    output.push_str(&selector[copied_until..]);
    (output, true)
}

fn join_fragments(fragments: &[StringFragment]) -> Result<String, String> {
    let mut output = String::new();
    for fragment in fragments {
        if fragment.inner.contains("${") {
            return Err("Stylist interpolation is not supported".into());
        }
        output.push_str(&fragment.inner);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::convert;

    #[test]
    fn converts_static_subset_with_stylist_scoping_and_stable_order() {
        let rules = convert(
            r#"
                color: red;
                color: blue;
                .label { font-size: 12px; }
                :hover { opacity: 0.5; }
                &.active { width: 10px; }
                :root.large { height: 20px; }
                header, footer { display: flex; }
            "#,
            "counter",
        )
        .unwrap();

        assert_eq!(
            serde_json::to_value(rules).unwrap(),
            json!([
                {
                    "type": "StyleRule",
                    "selectorText": { "value": ".counter" },
                    "style": [
                        { "name": "color", "value": "red" },
                        { "name": "color", "value": "blue" }
                    ],
                    "variables": {}
                },
                {
                    "type": "StyleRule",
                    "selectorText": { "value": ".counter .label" },
                    "style": [{ "name": "font-size", "value": "12px" }],
                    "variables": {}
                },
                {
                    "type": "StyleRule",
                    "selectorText": { "value": ".counter:hover" },
                    "style": [{ "name": "opacity", "value": "0.5" }],
                    "variables": {}
                },
                {
                    "type": "StyleRule",
                    "selectorText": { "value": ".counter.active" },
                    "style": [{ "name": "width", "value": "10px" }],
                    "variables": {}
                },
                {
                    "type": "StyleRule",
                    "selectorText": { "value": ".counter.large" },
                    "style": [{ "name": "height", "value": "20px" }],
                    "variables": {}
                },
                {
                    "type": "StyleRule",
                    "selectorText": { "value": ".counter header, .counter footer" },
                    "style": [{ "name": "display", "value": "flex" }],
                    "variables": {}
                }
            ])
        );
    }

    #[test]
    fn rejects_unsupported_or_empty_stylesheets() {
        for css in [
            "",
            ".empty {}",
            "@media screen { color: red; }",
            ".nested { @media screen { color: red; } }",
            "color: red; .outer { .inner { color: blue; } }",
            "color: ${color};",
        ] {
            assert!(convert(css, "counter").is_err(), "accepted {css:?}");
        }
    }

    #[test]
    fn rejects_invalid_classes() {
        for class in [
            "",
            "-",
            "-1counter",
            "1counter",
            "two classes",
            ".counter",
            "caf\u{e9}",
        ] {
            assert!(convert("color: red;", class).is_err(), "accepted {class:?}");
        }
    }

    #[test]
    fn does_not_rewrite_scope_tokens_inside_attribute_values() {
        let rules = convert(
            r#"[data-amp="a&b"], [data-root=":root"] { color: red; }"#,
            "counter",
        )
        .unwrap();

        assert_eq!(
            rules[0].selector_text.value,
            r#".counter [data-amp="a&b"], .counter [data-root=":root"]"#
        );
    }

    #[test]
    fn scopes_non_ascii_nested_selectors_without_panicking() {
        let rules = convert(".caf\u{e9} { color: red; }", "counter").unwrap();

        assert_eq!(rules[0].selector_text.value, ".counter .caf\u{e9}");
    }

    #[test]
    fn accepts_css_identifier_starts_in_the_supported_token_subset() {
        for class in ["counter", "_counter", "-counter", "--counter"] {
            assert!(convert("color: red;", class).is_ok(), "rejected {class:?}");
        }
    }
}
