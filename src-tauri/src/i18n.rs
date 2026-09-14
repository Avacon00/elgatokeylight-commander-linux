use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    System,
    De,
    En,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    De,
    En,
}
impl Language {
    pub fn resolve(self, system: Option<&str>) -> Locale {
        match self {
            Self::De => Locale::De,
            Self::En => Locale::En,
            Self::System => {
                let primary = system
                    .unwrap_or("")
                    .split(['-', '_', '.', '@'])
                    .next()
                    .unwrap_or("");
                if primary.eq_ignore_ascii_case("de") {
                    Locale::De
                } else {
                    Locale::En
                }
            }
        }
    }
    pub fn current(self) -> Locale {
        self.resolve(sys_locale::get_locale().as_deref())
    }
}
pub fn catalog(locale: Locale) -> &'static BTreeMap<String, String> {
    static DE: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    static EN: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    match locale {
        Locale::De => DE.get_or_init(|| {
            serde_json::from_str(include_str!("../../src/locales/de.json"))
                .expect("Bundled German translations")
        }),
        Locale::En => EN.get_or_init(|| {
            serde_json::from_str(include_str!("../../src/locales/en.json"))
                .expect("Bundled English translations")
        }),
    }
}
pub fn text(locale: Locale, key: &str) -> String {
    catalog(locale)
        .get(key)
        .or_else(|| catalog(Locale::En).get(key))
        .cloned()
        .unwrap_or_else(|| key.into())
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub key: String,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
    #[serde(default)]
    pub causes: Vec<Message>,
}
impl Message {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.into(),
            params: Default::default(),
            causes: vec![],
        }
    }
    pub fn param(mut self, key: &str, value: impl ToString) -> Self {
        self.params.insert(key.into(), value.to_string());
        self
    }
    pub fn cause(mut self, error: impl Into<Message>) -> Self {
        self.causes.push(error.into());
        self
    }
    pub fn render(&self, locale: Locale) -> String {
        let template = text(locale, &self.key);
        // Substitute template tokens once: device names/details may themselves contain braces.
        let mut output = String::new();
        let mut rest = template.as_str();
        while let Some(start) = rest.find('{') {
            output.push_str(&rest[..start]);
            rest = &rest[start..];
            if let Some(end) = rest.find('}') {
                let token = &rest[1..end];
                output.push_str(
                    self.params
                        .get(token)
                        .map(String::as_str)
                        .unwrap_or(&rest[..=end]),
                );
                rest = &rest[end + 1..];
            } else {
                break;
            }
        }
        output.push_str(rest);
        if !self.causes.is_empty() {
            output.push_str(": ");
            output.push_str(
                &self
                    .causes
                    .iter()
                    .map(|m| m.render(locale))
                    .collect::<Vec<_>>()
                    .join("; "),
            );
        }
        output
    }
}
impl From<String> for Message {
    fn from(details: String) -> Self {
        Self::new("error.technical").param("details", details)
    }
}
impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render(Locale::En))
    }
}
impl std::error::Error for Message {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_language_and_manual_override() {
        for value in ["de", "de_DE.UTF-8", "de-CH", "DE_at"] {
            assert_eq!(Language::System.resolve(Some(value)), Locale::De);
        }
        for value in [
            None,
            Some("en-US"),
            Some("fr_FR"),
            Some("C"),
            Some("unknown"),
        ] {
            assert_eq!(Language::System.resolve(value), Locale::En);
        }
        assert_eq!(Language::En.resolve(Some("de_DE")), Locale::En);
        assert_eq!(Language::De.resolve(Some("en_US")), Locale::De);
    }
    #[test]
    fn existing_message_retranslates_and_keeps_parameters_literal() {
        let message = Message::new("error.partial")
            .param("succeeded", 1)
            .param("failed", 1)
            .cause(
                Message::new("error.device")
                    .param("name", "My {name} light")
                    .cause(Message::new("error.invalid_setting")),
            );
        assert!(message.render(Locale::En).contains("Invalid light setting"));
        assert!(message
            .render(Locale::De)
            .contains("Ungültige Lampeneinstellung"));
        assert!(message.render(Locale::De).contains("My {name} light"));
    }
}
