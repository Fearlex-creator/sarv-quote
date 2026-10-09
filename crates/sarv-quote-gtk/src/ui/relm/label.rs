use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelTone {
    #[default]
    Body,
    Muted,
    Heading,
}

impl LabelTone {
    fn css_classes(self) -> &'static [&'static str] {
        match self {
            Self::Body => &["sarv-label"],
            Self::Muted => &["sarv-label", "sarv-label-muted"],
            Self::Heading => &["sarv-label", "sarv-label-heading"],
        }
    }
}

#[derive(Debug)]
pub enum SarvLabelMsg {
    SetText(String),
    SetTone(LabelTone),
}

pub struct SarvLabel {
    text: String,
    tone: LabelTone,
}

#[relm4::component(pub)]
impl SimpleComponent for SarvLabel {
    type Init = (String, LabelTone);
    type Input = SarvLabelMsg;
    type Output = ();

    view! {
        #[root]
        gtk::Label {
            set_label: &model.text,
            set_xalign: 0.0,
            set_wrap: true,
            set_selectable: false,
            add_css_class: "sarv-label",
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            text: init.0,
            tone: init.1,
        };
        for class in model.tone.css_classes() {
            if *class != "sarv-label" {
                root.add_css_class(class);
            }
        }
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            SarvLabelMsg::SetText(text) => self.text = text,
            SarvLabelMsg::SetTone(tone) => self.tone = tone,
        }
    }
}

/// Imperative factory matching Relm4 label chrome.
pub fn styled_label(text: &str, tone: LabelTone) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.set_selectable(false);
    for class in tone.css_classes() {
        label.add_css_class(class);
    }
    label
}
