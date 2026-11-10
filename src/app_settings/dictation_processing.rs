//! What happens to a dictation's text before it pastes: Global's steps and
//! the modes chosen by application or website.

use serde::{Deserialize, Serialize};

use super::TextReplacement;
use crate::dictation::vocabulary::{VocabularyEntry, serialize_saved};

/// One naming rule for explicit mode references in settings, the UI, and
/// runtime selection, including the reserved Global name.
pub(crate) fn mode_names_match(left: &str, right: &str) -> bool {
    left.trim().to_lowercase() == right.trim().to_lowercase()
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct DictationProcessingSettings {
    pub default_mode: DictationMode,
    pub modes: Vec<DictationMode>,
    /// An explicit mode for Labs › Second dictation shortcut (macOS).
    /// None keeps automatic app/site selection; Global selects Global alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_shortcut_mode: Option<String>,
}

impl Default for DictationProcessingSettings {
    fn default() -> Self {
        Self {
            default_mode: DictationMode {
                name: "Global".into(),
                ..Default::default()
            },
            modes: Vec::new(),
            second_shortcut_mode: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct DictationMode {
    pub name: String,
    pub applications: Vec<String>,
    pub browser_hosts: Vec<String>,
    pub replacements: Vec<TextReplacement>,
    pub transformations: Vec<String>,
    pub post_processing: DictationPostProcessing,
    /// Names heard and spelled here ahead of the shared vocabulary.
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_saved"
    )]
    pub vocabulary: Vec<VocabularyEntry>,
}

/// Persisted OpenCode rewrite settings for one mode.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct DictationPostProcessing {
    pub enabled: bool,
    pub prompt: String,
    /// Optional `provider/model` key from the OpenCode catalog.
    pub model: Option<String>,
    pub variant: Option<String>,
    pub deadline_seconds: u64,
}

impl Default for DictationPostProcessing {
    fn default() -> Self {
        Self {
            enabled: false,
            prompt: "Rewrite the transcript into clear, natural text while preserving the speaker's intended meaning.".into(),
            model: None,
            variant: None,
            deadline_seconds: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_names_have_one_case_insensitive_trimmed_comparison() {
        assert!(mode_names_match(" Reword ", "reword"));
        assert!(mode_names_match("Équipe", "équipe"));
        assert!(mode_names_match("Global", " GLOBAL "));
        assert!(!mode_names_match("Reword", "Reword more"));
    }

    #[test]
    fn older_settings_keep_automatic_second_shortcut_processing() {
        let settings: DictationProcessingSettings = serde_json::from_str("{}").unwrap();
        assert!(settings.second_shortcut_mode.is_none());
        assert!(
            serde_json::to_value(settings)
                .unwrap()
                .get("second_shortcut_mode")
                .is_none()
        );
    }
}
