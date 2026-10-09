use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum SarvInputMsg {
    SetText(String),
    SetPlaceholder(String),
    SetInvalid(bool),
    Changed(String),
    Activate,
}

#[derive(Debug)]
pub enum SarvInputOutput {
    Changed(String),
    Activate,
}

pub struct SarvInput {
    text: String,
    placeholder: String,
    pub invalid: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for SarvInput {
    type Init = (String, String);
    type Input = SarvInputMsg;
    type Output = SarvInputOutput;

    view! {
        #[root]
        gtk::Entry {
            set_text: &model.text,
            set_placeholder_text: Some(model.placeholder.as_str()),
            set_hexpand: true,
            add_css_class: "sarv-input",
            add_css_class: "sarv-entry",
            connect_changed[sender] => move |entry| {
                sender.input(SarvInputMsg::Changed(entry.text().to_string()));
            },
            connect_activate[sender] => move |_| {
                sender.input(SarvInputMsg::Activate);
            },
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let _ = &sender;
        let model = Self {
            text: init.0,
            placeholder: init.1,
            invalid: false,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SarvInputMsg::SetText(text) => self.text = text,
            SarvInputMsg::SetPlaceholder(placeholder) => self.placeholder = placeholder,
            SarvInputMsg::SetInvalid(invalid) => self.invalid = invalid,
            SarvInputMsg::Changed(text) => {
                self.text = text.clone();
                let _ = sender.output(SarvInputOutput::Changed(text));
            }
            SarvInputMsg::Activate => {
                let _ = sender.output(SarvInputOutput::Activate);
            }
        }
    }
}

/// Imperative factory matching Relm4 input chrome.
pub fn styled_entry(placeholder: &str) -> gtk::Entry {
    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some(placeholder));
    entry.set_hexpand(true);
    entry.add_css_class("sarv-input");
    entry.add_css_class("sarv-entry");
    entry
}
