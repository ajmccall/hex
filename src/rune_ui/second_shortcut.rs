//! Processing chosen explicitly by the macOS second dictation shortcut.

use rune::El;

use super::controls::{CardRow, Choice, Paint, icon_row, select};
use super::{Menu, Ui};
use crate::app_settings::mode_names_match;
use crate::desktop::i18n::tr;

pub(super) fn processing_row(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> CardRow {
    let selected = ui
        .settings
        .dictation_processing
        .second_shortcut_mode
        .as_deref();
    let mut choices = vec![
        choice(tr("Default"), None, selected),
        choice(tr("Global"), Some("Global"), selected),
    ];
    for mode in &ui.settings.dictation_processing.modes {
        let name = mode.name.trim();
        if !name.is_empty() && !mode_names_match(name, "Global") {
            choices.push(choice(name, Some(name), selected));
        }
    }
    let control = select(
        p,
        "second-shortcut-mode",
        tr("Processing"),
        selected.unwrap_or(tr("Default")),
        ui.open == Some(Menu::Pick("second-shortcut-mode")),
        choices,
        |ui| super::settings::toggle_menu(ui, Menu::Pick("second-shortcut-mode")),
        layer,
    );
    icon_row(
        p,
        "second-shortcut-mode",
        &super::icons::SPARKLES,
        tr("Processing"),
        None,
        control,
    )
    .into()
}

fn choice(label: &str, mode: Option<&str>, selected: Option<&str>) -> Choice {
    let picked = mode.map(str::to_owned);
    let selected = match (mode, selected) {
        (None, None) => true,
        (Some(mode), Some(selected)) => mode_names_match(mode, selected),
        _ => false,
    };
    Choice {
        label: label.into(),
        detail: None,
        selected,
        choose: Box::new(move |ui| {
            ui.open = None;
            ui.update(|settings| {
                settings.dictation_processing.second_shortcut_mode = picked.clone()
            });
        }),
    }
}
