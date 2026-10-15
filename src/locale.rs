//! The words of the original controls and desktop, in their five languages.

use crate::Language;
use std::sync::OnceLock;

type Catalog = serde_json::Map<String, serde_json::Value>;

const CONTROLS: [&str; 5] = [
    include_str!("../assets/locales/controls/en.json"),
    include_str!("../assets/locales/controls/zh-Hans.json"),
    include_str!("../assets/locales/controls/zh-Hant.json"),
    include_str!("../assets/locales/controls/ja.json"),
    include_str!("../assets/locales/controls/ko.json"),
];
const DESKTOP: [&str; 5] = [
    include_str!("../assets/locales/desktop/en.json"),
    include_str!("../assets/locales/desktop/zh-Hans.json"),
    include_str!("../assets/locales/desktop/zh-Hant.json"),
    include_str!("../assets/locales/desktop/ja.json"),
    include_str!("../assets/locales/desktop/ko.json"),
];

pub const LANGUAGES: [Language; 5] = [
    Language::English,
    Language::SimplifiedChinese,
    Language::TraditionalChinese,
    Language::Japanese,
    Language::Korean,
];
pub const LANGUAGE_NAMES: [&str; 5] = ["English", "简体中文", "繁體中文", "日本語", "한국어"];

impl Language {
    pub fn index(self) -> usize {
        match self {
            Self::English => 0,
            Self::SimplifiedChinese => 1,
            Self::TraditionalChinese => 2,
            Self::Japanese => 3,
            Self::Korean => 4,
        }
    }

    pub fn code(self) -> &'static str {
        ["en", "zh-Hans", "zh-Hant", "ja", "ko"][self.index()]
    }
}

fn read_catalogs(source: [&str; 5]) -> [Catalog; 5] {
    source.map(|json| serde_json::from_str(json).expect("bundled language catalog"))
}

fn lookup(catalogs: &[Catalog; 5], language: Language, key: &str) -> String {
    catalogs[language.index()]
        .get(key)
        .or_else(|| catalogs[0].get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or(key)
        .to_owned()
}

/// A control label. Braced parameters are left for the caller to replace.
pub fn tr(language: Language, key: &str) -> String {
    static CATALOGS: OnceLock<[Catalog; 5]> = OnceLock::new();
    lookup(
        CATALOGS.get_or_init(|| read_catalogs(CONTROLS)),
        language,
        key,
    )
}

pub fn desk(language: Language, key: &str) -> String {
    static CATALOGS: OnceLock<[Catalog; 5]> = OnceLock::new();
    lookup(
        CATALOGS.get_or_init(|| read_catalogs(DESKTOP)),
        language,
        key,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_language_keeps_the_original_control_and_desktop_keys() {
        for catalogs in [read_catalogs(CONTROLS), read_catalogs(DESKTOP)] {
            for key in catalogs[0].keys().filter(|key| !key.starts_with('@')) {
                for catalog in &catalogs {
                    assert!(
                        catalog.get(key).is_some_and(serde_json::Value::is_string),
                        "missing {key}"
                    );
                }
            }
        }
        assert_eq!(tr(Language::SimplifiedChinese, "cancel"), "取消");
        assert_eq!(
            desk(Language::English, "catText"),
            "The quick brown fox jumps over the lazy dog."
        );
    }
}

/// Format the plural and number expressions used by the bundled catalogs.
pub fn format_message(template: &str, arguments: &[(&str, String)]) -> String {
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        output.push_str(&rest[..start]);
        let mut depth = 0;
        let mut end = None;
        for (offset, ch) in rest[start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            output.push_str(&rest[start..]);
            return output;
        };
        let token = &rest[start + 1..end];
        let mut parts = token.splitn(3, ',');
        let name = parts.next().unwrap_or("").trim();
        let value = arguments
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
            .unwrap_or("");
        match parts.next().map(str::trim) {
            Some("plural") => {
                let choices = parts.next().unwrap_or("");
                let desired = if value == "1" { "one" } else { "other" };
                let selected = plural_branch(choices, &format!("={value}"))
                    .or_else(|| plural_branch(choices, desired))
                    .or_else(|| plural_branch(choices, "other"))
                    .unwrap_or("");
                output.push_str(&format_message(selected, arguments).replace('#', value));
            }
            Some("number") => {
                let style = parts.next().unwrap_or("");
                if style.contains(".0") {
                    output.push_str(&format!("{:.1}", value.parse::<f64>().unwrap_or(0.)));
                } else {
                    output.push_str(value);
                }
            }
            _ => output.push_str(value),
        }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    output
}
fn plural_branch<'a>(mut choices: &'a str, key: &str) -> Option<&'a str> {
    while let Some(start) = choices.find('{') {
        let name = choices[..start].trim();
        let mut depth = 0;
        let end = choices[start..].char_indices().find_map(|(offset, ch)| {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return Some(start + offset);
                }
            }
            None
        })?;
        if name == key {
            return Some(&choices[start + 1..end]);
        }
        choices = &choices[end + 1..];
    }
    None
}

#[cfg(test)]
mod format_tests {
    use super::format_message;
    #[test]
    fn original_plural_and_number_messages_are_formatted() {
        assert_eq!(
            format_message(
                "{count, plural, one {# item} other {# items}}",
                &[("count", "1".into())]
            ),
            "1 item"
        );
        assert_eq!(
            format_message(
                "{count, plural, one {# item} other {# items}}",
                &[("count", "4".into())]
            ),
            "4 items"
        );
        assert_eq!(
            format_message(
                "{size, number, ::.0} MB available",
                &[("size", "47.123".into())]
            ),
            "47.1 MB available"
        );
    }
}
