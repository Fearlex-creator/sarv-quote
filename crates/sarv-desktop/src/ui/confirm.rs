//! Reusable confirmation dialog for irreversible or guarded actions.

use std::rc::Rc;

use rune::{El, el, row};

use crate::app::State;
use crate::ui::glass::{Btn, DIALOG_SM, Ui};

#[derive(Clone)]
pub struct ConfirmChoice {
    pub key: String,
    pub label: String,
    pub danger: bool,
    pub action: Rc<dyn Fn(&mut State)>,
}

#[derive(Clone)]
pub struct Confirm {
    pub title: String,
    pub message: String,
    pub lines: Vec<(String, String)>,
    pub choices: Vec<ConfirmChoice>,
}

impl Confirm {
    pub fn one(
        title: impl Into<String>,
        message: impl Into<String>,
        lines: Vec<(String, String)>,
        confirm_key: impl Into<String>,
        confirm_label: impl Into<String>,
        danger: bool,
        action: impl Fn(&mut State) + 'static,
    ) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            lines,
            choices: vec![ConfirmChoice {
                key: confirm_key.into(),
                label: confirm_label.into(),
                danger,
                action: Rc::new(action),
            }],
        }
    }
}

pub fn mount(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let Some(c) = &s.confirm else {
        return;
    };
    let body = if c.lines.is_empty() {
        el()
    } else {
        let rows = c
            .lines
            .iter()
            .enumerate()
            .map(|(i, (label, value))| {
                row()
                    .key(format!("confirm-row-{i}"))
                    .w_full()
                    .min_h(36.0)
                    .px(12.0)
                    .py(8.0)
                    .gap(12.0)
                    .center_y()
                    .justify_between()
                    .child(ui.muted(label.clone()))
                    .child(ui.body(value.clone()))
            })
            .collect();
        ui.panel_rows("confirm-lines", rows)
    };

    let mut footer = vec![ui.btn(Btn::Ghost, "Cancel", None, |st| {
        st.confirm = None;
    })];
    for choice in &c.choices {
        let kind = if choice.danger { Btn::Danger } else { Btn::Primary };
        let action = choice.action.clone();
        footer.push(ui.button(kind, &choice.key, &choice.label, None, move |st| {
            let _ = st.confirm.take();
            action(st);
        }));
    }

    ui.dialog_described(
        layer,
        &mut Vec::new(),
        "confirm",
        &c.title,
        &c.message,
        true,
        DIALOG_SM,
        body,
        footer,
        |st| {
            st.confirm = None;
        },
    );
}
