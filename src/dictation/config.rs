use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

use crate::app_settings::DictationProcessingSettings;
use crate::dictation::processor::{Profile, Profiles};
use crate::dictation::text_replacements::ReplacementSet;

static DICTATION_PROFILES: OnceLock<RwLock<Profiles>> = OnceLock::new();
static READS_BROWSER: AtomicBool = AtomicBool::new(false);
static READS_WINDOW_TITLE: AtomicBool = AtomicBool::new(false);
static SECOND_SHORTCUT_MODE: OnceLock<RwLock<Option<String>>> = OnceLock::new();

/// The name is latched by the shortcut that starts a capture, not read again
/// on release or when a queued dictation reaches the output worker.
pub(crate) fn second_shortcut_mode() -> Option<String> {
    SECOND_SHORTCUT_MODE
        .get_or_init(Default::default)
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

/// Whether anything the user set up looks at the page: a mode chosen by
/// website, an OpenCode rewrite (its prompt names the site), or a script
/// transformation (it receives the page address). Otherwise Hex never asks a
/// browser for its page, so macOS never shows an Automation prompt for it.
pub fn reads_browser() -> bool {
    READS_BROWSER.load(Ordering::Acquire)
}

/// Whether a script transformation is set up, the only reader of the front
/// window's title. Otherwise Hex never reads it.
pub fn reads_window_title() -> bool {
    READS_WINDOW_TITLE.load(Ordering::Acquire)
}

fn uses_scripts(settings: &DictationProcessingSettings) -> bool {
    std::iter::once(&settings.default_mode)
        .chain(&settings.modes)
        .flat_map(|mode| &mode.transformations)
        .any(|id| !crate::personal_commands::is_builtin_transformation(id))
}

fn uses_browser(settings: &DictationProcessingSettings) -> bool {
    crate::dictation::modes::uses_page(settings)
}

pub fn dictation_profiles() -> Profiles {
    dictation_profiles_lock()
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

pub fn update_dictation_profiles(settings: &DictationProcessingSettings) {
    let profiles = build_dictation_profiles(settings);
    *SECOND_SHORTCUT_MODE
        .get_or_init(Default::default)
        .write()
        .unwrap_or_else(|error| error.into_inner()) = settings.second_shortcut_mode.clone();
    READS_BROWSER.store(uses_browser(settings), Ordering::Release);
    READS_WINDOW_TITLE.store(uses_scripts(settings), Ordering::Release);
    *dictation_profiles_lock()
        .write()
        .unwrap_or_else(|error| error.into_inner()) = profiles;
}

fn dictation_profiles_lock() -> &'static RwLock<Profiles> {
    DICTATION_PROFILES.get_or_init(|| {
        RwLock::new(build_dictation_profiles(
            &DictationProcessingSettings::default(),
        ))
    })
}

fn build_dictation_profiles(settings: &DictationProcessingSettings) -> Profiles {
    let global = &settings.default_mode;
    let mut profiles = Profiles::new(
        Profile::configured(
            "Global",
            ReplacementSet::default(),
            global.transformations.clone(),
            &global.post_processing,
        )
        .vocabulary(global.vocabulary.clone()),
    )
    .global_replacements(ReplacementSet::new(&global.replacements));
    for mode in &settings.modes {
        let profile = Profile::from_mode(mode);
        profiles = profiles.named(profile.clone());
        for application in &mode.applications {
            profiles = profiles.application(application.clone(), profile.clone());
        }
        for host in &mode.browser_hosts {
            profiles = profiles.browser_host(host.clone(), profile.clone());
        }
    }
    profiles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictation::context::ContextSnapshot;

    #[test]
    fn second_shortcut_selects_a_mode_without_app_rules_and_keeps_global_cleanup() {
        use crate::app_settings::{DictationMode, TextReplacement};
        let mut settings = DictationProcessingSettings {
            second_shortcut_mode: Some("Reword".into()),
            modes: vec![DictationMode {
                name: "Reword".into(),
                replacements: vec![TextReplacement {
                    matched_phrase: "tomorrow".into(),
                    output: "next week".into(),
                }],
                transformations: vec!["lowercase".into()],
                ..DictationMode::default()
            }],
            ..DictationProcessingSettings::default()
        };
        settings.default_mode.replacements = vec![TextReplacement {
            matched_phrase: "um".into(),
            output: String::new(),
        }];
        let profiles = build_dictation_profiles(&settings);
        let context = ContextSnapshot::default();
        let process = |context: &ContextSnapshot| {
            profiles.process_cancellable("Um, ship tomorrow.", context, &AtomicBool::new(false))
        };
        assert_eq!(process(&context).text, "Ship tomorrow.");
        assert!(!profiles.processes(&context));
        let explicit = ContextSnapshot {
            processing_mode: settings.second_shortcut_mode.clone(),
            ..context
        };
        let processed = process(&explicit);
        assert_eq!(processed.text, "Ship next week.");
        assert_eq!(processed.transformations, ["lowercase"]);
        assert!(profiles.processes(&explicit));
    }

    #[test]
    fn reword_failure_keeps_filler_cleanup_and_global_replacements() {
        let mode: crate::app_settings::DictationMode =
            serde_json::from_str(include_str!("../../resources/reword-mode.json")).unwrap();
        assert!(mode.post_processing.enabled);
        assert!(mode.post_processing.model.is_none());
        let mut settings = DictationProcessingSettings {
            modes: vec![mode],
            ..DictationProcessingSettings::default()
        };
        settings.default_mode.replacements = vec![crate::app_settings::TextReplacement {
            matched_phrase: "ship".into(),
            output: "send".into(),
        }];
        crate::labs::apply(&crate::app_settings::LabsSettings {
            filler_words: true,
            ..Default::default()
        });
        let profiles = build_dictation_profiles(&settings);
        let context = ContextSnapshot {
            processing_mode: Some("Reword".into()),
            ..Default::default()
        };
        // Cancellation before discovery avoids any live service/provider call.
        let processed = profiles.process_cancellable(
            "Um, we should uh ship it.",
            &context,
            &AtomicBool::new(true),
        );
        crate::labs::apply(&crate::app_settings::LabsSettings::default());
        assert_eq!(processed.text, "We should send it.");
        let observation = processed.observation.unwrap();
        assert_eq!(observation.profile, "Reword");
        assert!(observation.fallback.is_some());
    }

    #[test]
    fn an_explicit_mode_wins_over_websites_and_missing_modes_fall_back_to_global() {
        use crate::app_settings::DictationMode;
        let settings = DictationProcessingSettings {
            modes: vec![DictationMode {
                name: "Slack".into(),
                browser_hosts: vec!["slack.com".into()],
                transformations: vec!["lowercase".into()],
                ..DictationMode::default()
            }],
            ..DictationProcessingSettings::default()
        };
        let profiles = build_dictation_profiles(&settings);
        let mut context = ContextSnapshot {
            browser_url: Some("https://slack.com".parse().unwrap()),
            ..ContextSnapshot::default()
        };
        assert!(profiles.processes(&context));
        for name in ["Global", "Deleted mode"] {
            context.processing_mode = Some(name.into());
            assert!(!profiles.processes(&context));
        }
        context.processing_mode = Some(" slack ".into());
        assert!(profiles.processes(&context));
    }

    #[test]
    fn configured_mode_profiles_match_application_context() {
        let settings = DictationProcessingSettings {
            modes: vec![crate::app_settings::DictationMode {
                applications: vec!["Slack".into()],
                transformations: vec!["lowercase".into()],
                ..crate::app_settings::DictationMode::default()
            }],
            ..DictationProcessingSettings::default()
        };
        let context = ContextSnapshot {
            application: Some("Slack".into()),
            ..ContextSnapshot::default()
        };

        assert!(build_dictation_profiles(&settings).processes(&context));
    }

    #[test]
    fn global_corrections_run_before_an_app_modes_own() {
        use crate::app_settings::DictationMode;
        use crate::dictation::text_replacements::TextReplacement;
        let pair = |phrase: &str, output: &str| TextReplacement {
            matched_phrase: phrase.into(),
            output: output.into(),
        };
        let mut settings = DictationProcessingSettings {
            modes: vec![DictationMode {
                applications: vec!["Slack".into()],
                replacements: vec![pair("OpenCode", "OpenCode :rocket:")],
                ..DictationMode::default()
            }],
            ..DictationProcessingSettings::default()
        };
        settings.default_mode.replacements = vec![pair("open code", "OpenCode")];
        let profiles = build_dictation_profiles(&settings);
        let process = |application: &str| {
            let context = ContextSnapshot {
                application: Some(application.into()),
                ..ContextSnapshot::default()
            };
            profiles
                .process_cancellable(
                    "I like open code",
                    "en",
                    &context,
                    &AtomicBool::new(false),
                    crate::dictation::processor::Source::Dictation,
                )
                .text
        };

        assert_eq!(process("Slack"), "I like OpenCode :rocket:");
        assert_eq!(process("Notes"), "I like OpenCode");
    }

    #[test]
    fn an_app_mode_has_its_own_vocabulary() {
        use crate::app_settings::DictationMode;
        use crate::dictation::vocabulary::VocabularyEntry;
        let term = |term: &str| VocabularyEntry {
            term: term.into(),
            sounds_like: Vec::new(),
        };
        let settings = DictationProcessingSettings {
            modes: vec![DictationMode {
                applications: vec!["Zed".into()],
                vocabulary: vec![term("useEffect")],
                ..DictationMode::default()
            }],
            ..DictationProcessingSettings::default()
        };
        let profiles = build_dictation_profiles(&settings);
        let in_app = |application: &str| {
            profiles.vocabulary_for(&ContextSnapshot {
                application: Some(application.into()),
                ..ContextSnapshot::default()
            })
        };

        assert_eq!(in_app("Zed"), [term("useEffect")]);
        assert!(in_app("Notes").is_empty());
    }

    #[test]
    fn browsers_are_asked_only_when_something_uses_the_page() {
        use crate::app_settings::{DictationMode, DictationPostProcessing};
        let with = |mode: DictationMode| DictationProcessingSettings {
            modes: vec![mode],
            ..DictationProcessingSettings::default()
        };

        assert!(!uses_browser(&DictationProcessingSettings::default()));
        assert!(!uses_browser(&with(DictationMode {
            applications: vec!["Slack".into()],
            transformations: vec!["lowercase".into()],
            ..DictationMode::default()
        })));
        assert!(uses_browser(&with(DictationMode {
            browser_hosts: vec!["x.com".into()],
            ..DictationMode::default()
        })));
        let script = with(DictationMode {
            applications: vec!["Slack".into()],
            transformations: vec!["my-script".into()],
            ..DictationMode::default()
        });
        assert!(uses_browser(&script));
        assert!(uses_scripts(&script));
        let rewrite = with(DictationMode {
            applications: vec!["Slack".into()],
            post_processing: DictationPostProcessing {
                enabled: true,
                ..DictationPostProcessing::default()
            },
            ..DictationMode::default()
        });
        assert!(uses_browser(&rewrite));
        assert!(!uses_scripts(&rewrite));

        let global = |transformation: &str| DictationProcessingSettings {
            default_mode: DictationMode {
                transformations: vec![transformation.into()],
                ..DictationMode::default()
            },
            ..DictationProcessingSettings::default()
        };
        assert!(uses_scripts(&global("my-script")));
        assert!(!uses_scripts(&global("lowercase")));
    }
}
