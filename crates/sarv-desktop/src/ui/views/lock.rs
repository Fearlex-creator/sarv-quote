//! App lock screen: gates access to the books and unseals encrypted data on unlock.

use rune::{Cx, El, col, el, icon, row, text};

use crate::app::State;
use crate::i18n::Lang;
use crate::security::{self, MAX_ATTEMPTS_BEFORE_LOCK, VerifyOutcome};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;

#[derive(Default)]
pub struct LockState {
    pub error: String,
    pub revealed: bool,
    pub failed_attempts: u32,
    pub locked_until_unix: i64,
}

impl LockState {
    pub fn sync_from_disk(&mut self, dir: &std::path::Path) {
        if let Ok(Some(rec)) = security::load(dir) {
            self.failed_attempts = rec.failed_attempts;
            self.locked_until_unix = rec.locked_until_unix;
        } else {
            self.failed_attempts = 0;
            self.locked_until_unix = 0;
        }
    }

    pub fn remaining_lockout_secs(&self, now: i64) -> i64 {
        if self.locked_until_unix > now {
            self.locked_until_unix - now
        } else {
            0
        }
    }
}

pub fn format_duration(s: &State, secs: i64) -> String {
    let secs = secs.max(0);
    let mins = secs / 60;
    let rem = secs % 60;
    let secs_str = s.fmt.number(rem);
    if mins == 0 {
        return s.fmt.tf("{n} seconds", &[("n", &secs_str)]);
    }
    let mins_str = s.fmt.number(mins);
    if rem == 0 {
        s.fmt.tf("{n} minutes", &[("n", &mins_str)])
    } else {
        s.fmt.tf("{m} minutes and {s} seconds", &[("m", &mins_str), ("s", &secs_str)])
    }
}

pub fn format_outcome_error(s: &State, outcome: VerifyOutcome) -> String {
    match outcome {
        VerifyOutcome::Unlocked(_) => String::new(),
        VerifyOutcome::LockedOut { remaining_secs, .. } => {
            let dur = format_duration(s, remaining_secs);
            s.fmt.tf("Too many failed attempts. Try again in {time}.", &[("time", &dur)])
        }
        VerifyOutcome::WrongPassword {
            attempts_until_lock,
            lockout_secs,
            ..
        } => {
            if lockout_secs > 0 {
                let dur = format_duration(s, lockout_secs);
                s.fmt.tf("Too many failed attempts. Locked for {time}.", &[("time", &dur)])
            } else {
                let rem = s.fmt.number(attempts_until_lock as i64);
                s.fmt
                    .tf("Incorrect password. {n} attempts remaining before temporary lock.", &[("n", &rem)])
            }
        }
    }
}

pub fn attempt_unlock(s: &mut State) {
    let password = s.forms.get("lock-password");
    if password.is_empty() {
        s.lock_state.error = s.fmt.t("Enter your password.");
        return;
    }
    let now = security::now_unix();
    match security::verify_and_record(&s.engine.dir, &password, now) {
        Ok(VerifyOutcome::Unlocked(dek)) => {
            if let Err(e) = security::unseal_workspace_data(&s.engine.dir, &dek) {
                s.lock_state.error = e;
                return;
            }
            s.active_dek = Some(zeroize::Zeroizing::new(dek));
            s.locked = false;
            s.lock_state.error.clear();
            s.lock_state.revealed = false;
            s.lock_state.failed_attempts = 0;
            s.lock_state.locked_until_unix = 0;
            s.forms.clear(&["lock-password"]);
            s.refresh_and_apply_settings();
            s.startup_summary_open = s.books.has_company() && s.books.initialized();
            crate::logfile::line("INFO", "app unlocked");
        }
        Ok(outcome) => {
            s.forms.clear(&["lock-password"]);
            s.lock_state.sync_from_disk(&s.engine.dir);
            s.lock_state.error = format_outcome_error(s, outcome);
        }
        Err(e) => {
            s.lock_state.error = e;
        }
    }
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui) -> El {
    let remaining = s.lock_state.remaining_lockout_secs(security::now_unix());
    let fails = s.lock_state.failed_attempts;

    let badge_icon = row()
        .size(52.0, 52.0)
        .center()
        .radius(26.0)
        .fill(ui.k.c.layer_02)
        .border(1.0, ui.k.c.border_muted)
        .child(icon(&icons::LOCK, 24.0).color(ui.k.c.info.icon));

    let header = col()
        .w_full()
        .gap(8.0)
        .center()
        .child(badge_icon)
        .child(ui.title(ui.t("Unlock Sarv")))
        .child(ui.muted(ui.t("Your books and documents are encrypted on this device.")));

    let pass_field = ui.secret_field_submit(s, "lock-password", &ui.t("Password"), s.lock_state.revealed, |st, tx| {
        attempt_unlock(st);
        if st.lock_state.remaining_lockout_secs(security::now_unix()) > 0 {
            tx.wake_after(std::time::Duration::from_secs(1));
        }
    });

    let toggle_label = if s.lock_state.revealed { "Hide" } else { "Show" };
    let input_row = row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(pass_field.grow(1.0).min_w(0.0))
        .child(ui.btn(Btn::Ghost, toggle_label, None, |st| {
            st.lock_state.revealed = !st.lock_state.revealed;
        }));

    let mut card_body = col().w_full().gap(16.0).child(header).child(ui.labeled("Password", input_row));

    if fails > 0 && remaining == 0 && fails < MAX_ATTEMPTS_BEFORE_LOCK {
        let mut dots = row().gap(6.0).center_y();
        for i in 0..MAX_ATTEMPTS_BEFORE_LOCK {
            let color = if i < fails { ui.k.c.danger.icon } else { ui.k.c.border_muted };
            dots = dots.child(el().size(8.0, 8.0).radius(4.0).fill(color));
        }
        let used = s.fmt.number(fails as i64);
        let max = s.fmt.number(MAX_ATTEMPTS_BEFORE_LOCK as i64);
        card_body = card_body.child(
            row()
                .w_full()
                .justify_between()
                .center_y()
                .child(ui.muted(ui.t("Failed attempts")))
                .child(row().gap(8.0).center_y().child(dots).child(ui.muted(format!("{used} / {max}")))),
        );
    }

    let status_msg = if remaining > 0 {
        let dur = format_duration(s, remaining);
        s.fmt.tf("Too many failed attempts. Try again in {time}.", &[("time", &dur)])
    } else {
        s.lock_state.error.clone()
    };

    if !status_msg.is_empty() {
        let sc = &ui.k.c.danger;
        card_body = card_body.child(
            row()
                .key("lock-error")
                .w_full()
                .gap(8.0)
                .px(12.0)
                .py(10.0)
                .center_y()
                .radius(ui.k.radius.md)
                .fill(sc.bg)
                .border(1.0, sc.border)
                .child(icon(&icons::WARNING, 16.0).color(sc.icon))
                .child(text(status_msg).font(ui.k.ty.caption()).color(sc.fg).grow(1.0).min_w(0.0)),
        );
    }

    let unlock_btn = ui.btn(Btn::Primary, "Unlock", Some(&icons::CHECK), attempt_unlock).w_full();

    card_body = card_body.child(unlock_btn);

    let card = ui.k.elevate(
        col()
            .key("lock-card")
            .w_full()
            .max_w(420.0)
            .padding(24.0)
            .radius(ui.k.radius.lg)
            .fill(ui.k.c.layer_01)
            .border(1.0, ui.k.c.border_muted)
            .child(card_body),
        &ui.k.elevation.raised,
    );

    let lang_label = match s.lang() {
        Lang::Fa => "English",
        Lang::En => "فارسی",
    };
    let theme_icon = if s.dark { &icons::SUN } else { &icons::MOON };
    let theme_label = if s.dark { "Light" } else { "Dark" };

    let footer = row()
        .gap(8.0)
        .center()
        .child(ui.button(Btn::Ghost, "lock-lang", lang_label, Some(&icons::GLOBE), |st| {
            let next = st.lang().other();
            st.set_lang(next);
        }))
        .child(ui.btn(Btn::Ghost, theme_label, Some(theme_icon), |st| {
            st.dark = !st.dark;
        }));

    col()
        .key("lock-screen")
        .w_full()
        .h_full()
        .padding(24.0)
        .gap(16.0)
        .center()
        .fill(ui.k.c.page)
        .child(card)
        .child(footer)
}
