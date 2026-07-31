/// Canonical identifiers derived from a parameter configuration type name.
///
/// Both parameter proc macros use this value so the macro that generates field
/// descriptors and the macro that refers to them cannot drift independently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterGroupNaming {
    base_name: String,
    method_prefix: String,
}

impl ParameterGroupNaming {
    /// Derives the naming contract for `config_type_name`.
    #[must_use]
    pub fn new(config_type_name: &str) -> Self {
        let base_name = config_type_name.strip_suffix("Config").unwrap_or(config_type_name).to_owned();
        let method_prefix = snake_case(&base_name);

        Self { base_name, method_prefix }
    }

    /// Returns the source-style group name without its optional `Config` suffix.
    #[must_use]
    pub fn base_name(&self) -> &str {
        &self.base_name
    }

    /// Returns the snake-case prefix used by generated accessor methods.
    #[must_use]
    pub fn method_prefix(&self) -> &str {
        &self.method_prefix
    }

    /// Returns the exhaustive changed-field mapper trait name for this group.
    #[must_use]
    pub fn changed_field_mapper_name(&self) -> String {
        format!("{}ChangedFieldMapper", self.base_name())
    }

    /// Returns the canonical descriptor type name for `field_name`.
    #[must_use]
    pub fn field_descriptor_name(&self, field_name: &str) -> String {
        format!("{}{}Field", upper_camel_case(self.method_prefix()), upper_camel_case(field_name))
    }
}

fn upper_camel_case(name: &str) -> String {
    let mut out = String::new();
    let mut uppercase_next = true;

    for ch in name.chars() {
        if ch == '_' {
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            out.push(ch.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            out.push(ch);
        }
    }

    out
}

fn snake_case(name: &str) -> String {
    let chars = name.chars().collect::<Vec<_>>();
    let mut out = String::new();

    for (index, ch) in chars.iter().copied().enumerate() {
        if ch.is_uppercase() && index > 0 {
            let previous = chars[index - 1];
            let next = chars.get(index + 1).copied();
            if previous.is_lowercase() || previous.is_ascii_digit() || next.is_some_and(char::is_lowercase) {
                out.push('_');
            }
        }
        out.push(ch.to_ascii_lowercase());
    }

    out
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
