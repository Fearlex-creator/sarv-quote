//! Recovery screen shown when the books fail to open or validate on launch.

use rune::{Cx, El, col, row};
use serde_json::json;

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;
use crate::ui::views::settings::{backup_list, preview_summary};

#[derive(Default)]
pub struct RecoveryState {
    pub backups: serde_json::Value,
    pub selected_name: String,
    pub preview_note: String,
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, _layer: &mut Vec<El>) -> El {
    let list = backup_list(ui, &s.recovery.backups, &s.recovery.selected_name, |st, name| {
        match st.query("backup_restore_preview", json!({ "filename": name })) {
            Ok(v) => {
                st.recovery.selected_name = name;
                st.recovery.preview_note = preview_summary(&v);
            }
            Err(e) => st.toast_err(e),
        }
    });

    let mut card = ui
        .kit_card()
        .title(ui.t("Your books could not be opened"), Some(&icons::SHIELD))
        .description(s.books.error.clone())
        .child(ui.muted(crate::i18n::isolate(&s.engine.dir.display().to_string())))
        .child(list);
    if !s.recovery.preview_note.is_empty() {
        card = card.child(ui.muted(s.recovery.preview_note.clone()));
    }
    let card = card.build(&ui.k);

    let actions = row()
        .w_full()
        .gap(8.0)
        .child(ui.btn(Btn::Glass, "Try again", None, |st| st.refresh()))
        .child(ui.btn(Btn::Danger, "Restore this backup", Some(&icons::DOWNLOAD), |st| {
            let name = st.recovery.selected_name.clone();
            if name.is_empty() {
                st.toast(Tone::Info, st.fmt.t("No backups yet. Create one before your first closing."));
                return;
            }
            let payload = json!({
                "filename": name,
                "confirmed": true,
            });
            if st.run("recover_from_backup", payload, Some("Saved")).is_ok() {
                st.recovery = Default::default();
                st.books.company_id.clear();
                st.books.period_id.clear();
                st.sheets.loaded = false;
                st.pos = Default::default();
                st.refresh();
                st.apply_settings();
            }
        }));

    let hero = col().w_full().max_w(560.0).gap(16.0).child(card).child(actions);
    col().key("recovery-screen").w_full().h_full().center().padding(32.0).child(hero)
}
