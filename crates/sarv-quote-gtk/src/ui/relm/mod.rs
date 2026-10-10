//! Relm4 primitives with isolated model + view. Embed via `Controller` or
//! the thin factory helpers that apply the same `.sarv-*` token classes.
//! Full components are ready for page migration; factories drive current UI.

#![allow(dead_code)]

mod button;
mod card;
mod input;
mod label;

pub use button::{
    styled_button, ButtonVariant, SarvButton, SarvButtonMsg, SarvButtonOutput,
};
pub use card::{styled_card, SarvCard, SarvCardInit, SarvCardMsg};
pub use input::{styled_entry, SarvInput, SarvInputMsg, SarvInputOutput};
pub use label::{styled_label, LabelTone, SarvLabel, SarvLabelMsg};
