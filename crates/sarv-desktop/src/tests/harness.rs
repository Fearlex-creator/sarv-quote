#![allow(dead_code)]

use crate::app::{State, Tone};
use crate::{data, ui};
use rune::{Env, Runtime};

pub fn make_test_runtime() -> Runtime<State> {
    let env = Env {
        system_dark: true,
        scale: 2.0,
    };
    let mut rt = Runtime::new(State::with_engine(data::Engine::demo(), true), ui::root_view, (1180.0, 780.0), env);
    rt.frame(0.0);
    rt
}

pub fn click(rt: &mut Runtime<State>, key: &str) {
    let target = key.to_string();
    rt.update(|_, tx| tx.scroll_into_view(target));
    rt.frame(1.0 / 60.0);
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing element `{key}`"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

pub fn blow(rt: &Runtime<State>, what: &str) -> String {
    let msgs: Vec<_> = rt.state.toasts.iter().map(|t| format!("{:?}:{}", t.tone, t.message)).collect();
    format!("{what}; toasts={msgs:?}")
}

pub fn type_into(rt: &mut Runtime<State>, form_key: &str, text: &str) {
    rt.state.forms.set(form_key, text);
    rt.frame(1.0 / 60.0);
}

pub fn danger_toasts(rt: &Runtime<State>) -> Vec<String> {
    rt.state
        .toasts
        .iter()
        .filter(|t| t.tone == Tone::Danger)
        .map(|t| t.message.clone())
        .collect()
}

pub fn assert_no_danger(rt: &Runtime<State>) {
    let dangers = danger_toasts(rt);
    assert!(dangers.is_empty(), "unexpected danger toasts: {dangers:?}");
}

pub fn count(rt: &Runtime<State>, key: &str) -> usize {
    rt.state.books.count(key)
}

pub fn trial_balance_is_balanced(rt: &Runtime<State>) -> bool {
    let trial = &rt.state.books.report["trial"];
    let debit = data::s(&trial["total_debit"]).parse::<i64>();
    let credit = data::s(&trial["total_credit"]).parse::<i64>();
    matches!((debit, credit), (Ok(d), Ok(c)) if d == c)
}

pub fn books_validate(rt: &Runtime<State>) {
    sarv_quote::store::Store::open(&rt.state.engine.dir).expect("books reopen and validate");
}
