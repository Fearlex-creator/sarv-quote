use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
    Outline,
    Ghost,
    Destructive,
}

impl ButtonVariant {
    fn css_class(self) -> &'static str {
        match self {
            Self::Default => "sarv-btn",
            Self::Primary => "sarv-btn-primary",
            Self::Outline => "sarv-btn-outline",
            Self::Ghost => "sarv-btn-ghost",
            Self::Destructive => "sarv-btn-destructive",
        }
    }
}

#[derive(Debug)]
pub enum SarvButtonMsg {
    SetLabel(String),
    SetEnabled(bool),
    SetVariant(ButtonVariant),
    Clicked,
}

#[derive(Debug)]
pub enum SarvButtonOutput {
    Clicked,
}

pub struct SarvButton {
    label: String,
    variant: ButtonVariant,
    enabled: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for SarvButton {
    type Init = (String, ButtonVariant);
    type Input = SarvButtonMsg;
    type Output = SarvButtonOutput;

    view! {
        #[root]
        gtk::Button {
            set_label: &model.label,
            set_sensitive: model.enabled,
            add_css_class: "sarv-btn",
            add_css_class: model.variant.css_class(),
            connect_clicked => SarvButtonMsg::Clicked,
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            label: init.0,
            variant: init.1,
            enabled: true,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SarvButtonMsg::SetLabel(label) => self.label = label,
            SarvButtonMsg::SetEnabled(enabled) => self.enabled = enabled,
            SarvButtonMsg::SetVariant(variant) => self.variant = variant,
            SarvButtonMsg::Clicked => {
                let _ = sender.output(SarvButtonOutput::Clicked);
            }
        }
    }
}

/// Imperative factory — same chrome as the Relm4 component, for existing builders.
pub fn styled_button(label: &str, variant: ButtonVariant) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("sarv-btn");
    button.add_css_class(variant.css_class());
    button
}
