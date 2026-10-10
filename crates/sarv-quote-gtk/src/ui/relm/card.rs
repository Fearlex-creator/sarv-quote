use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Default)]
pub struct SarvCardInit {
    pub title: Option<String>,
}

#[derive(Debug)]
pub enum SarvCardMsg {
    SetTitle(Option<String>),
}

pub struct SarvCard {
    title: Option<String>,
}

#[relm4::component(pub)]
impl SimpleComponent for SarvCard {
    type Init = SarvCardInit;
    type Input = SarvCardMsg;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
            add_css_class: "sarv-card",
            add_css_class: "card",
            add_css_class: "component-card",

            gtk::Label {
                set_xalign: 0.0,
                set_selectable: false,
                add_css_class: "sarv-label-heading",
                #[watch]
                set_visible: model.title.is_some(),
                #[watch]
                set_label: model.title.as_deref().unwrap_or(""),
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_hexpand: true,
                set_vexpand: true,
            },
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { title: init.title };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            SarvCardMsg::SetTitle(title) => self.title = title,
        }
    }
}

/// Imperative factory matching Relm4 card chrome (spacing on the 4/8/12/16 grid).
pub fn styled_card() -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.add_css_class("sarv-card");
    card.add_css_class("card");
    card.add_css_class("component-card");
    card
}
