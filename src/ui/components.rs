use gtk::prelude::*;

pub(super) fn column() -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, 8)
}
pub(super) fn row() -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, 8)
}
pub(super) fn label(s: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(s));
    l.set_xalign(0.0);
    l.set_wrap(true);
    l.set_selectable(true);
    l
}
pub(super) fn heading(s: &str) -> gtk::Label {
    let l = label(s);
    l.set_selectable(false);
    l.add_css_class("heading");
    l
}
pub(super) fn button(s: &str) -> gtk::Button {
    gtk::Button::with_label(s)
}
pub(super) fn accessible_name(widget: &impl IsA<gtk::Widget>, name: &str) {
    widget
        .as_ref()
        .update_property(&[gtk::accessible::Property::Label(name)]);
}
pub(super) fn entry(placeholder: &str) -> gtk::Entry {
    let e = gtk::Entry::new();
    e.set_placeholder_text(Some(placeholder));
    e.set_hexpand(true);
    e
}
/// Native invalid styling and an accessible explanation accompany the same
/// domain validation used by persistence. The tooltip also exposes the limit.
pub(super) fn entry_validity(entry: &gtk::Entry, error: Option<&str>, hint: &str) {
    let explanation = error.unwrap_or(hint);
    if error.is_some() {
        entry.add_css_class("error");
    } else {
        entry.remove_css_class("error");
    }
    entry.set_tooltip_text(Some(explanation));
    entry.update_state(&[gtk::accessible::State::Invalid(if error.is_some() {
        gtk::AccessibleInvalidState::True
    } else {
        gtk::AccessibleInvalidState::False
    })]);
    entry.update_property(&[gtk::accessible::Property::Description(explanation)]);
}
pub(super) fn scroll(w: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .child(w)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build()
}
pub(super) fn clear(b: &gtk::Box) {
    while let Some(c) = b.first_child() {
        b.remove(&c);
    }
}
pub(super) fn card() -> gtk::Box {
    let b = column();
    b.add_css_class("card");
    b.add_css_class("component-card");
    b
}

/// Named states are separate from persisted document statuses.
/// A green label must never itself imply that a quote has been saved or sent.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(super) enum StatusTone {
    Neutral,
    Success,
    Warning,
    Error,
}

impl StatusTone {
    fn class(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// Keep a persistent caption above a control; placeholders are only examples.
#[allow(dead_code)]
pub(super) fn field(caption: &str, control: &impl IsA<gtk::Widget>) -> gtk::Box {
    let b = column();
    b.set_spacing(4);
    b.add_css_class("field");
    let caption = label(caption);
    caption.set_selectable(false);
    caption.add_css_class("field-label");
    caption.add_css_class("caption-heading");
    caption.set_mnemonic_widget(Some(control));
    control
        .as_ref()
        .update_relation(&[gtk::accessible::Relation::LabelledBy(&[
            caption.upcast_ref()
        ])]);
    b.append(&caption);
    b.append(control);
    b
}

#[allow(dead_code)]
pub(super) fn section(title: &str, subtitle: Option<&str>) -> gtk::Box {
    let b = column();
    b.add_css_class("section");
    b.append(&heading(title));
    if let Some(text) = subtitle {
        let subtitle = label(text);
        subtitle.add_css_class("muted");
        subtitle.add_css_class("caption");
        subtitle.add_css_class("dimmed");
        b.append(&subtitle);
    }
    b
}

#[allow(dead_code)]
pub(super) fn toolbar() -> gtk::Box {
    let b = row();
    b.add_css_class("document-toolbar");
    b.add_css_class("toolbar");
    b.set_valign(gtk::Align::Center);
    b
}

#[allow(dead_code)]
pub(super) fn page_title(text: &str) -> gtk::Label {
    let title = heading(text);
    title.remove_css_class("heading");
    title.add_css_class("page-title");
    title.add_css_class("title-2");
    title
}

/// For simpler lists; the quote editor uses a GtkGrid for shared column widths.
#[allow(dead_code)]
pub(super) fn table_header(captions: &[&str]) -> gtk::Box {
    let b = row();
    b.set_spacing(0);
    b.add_css_class("table-header");
    for caption in captions {
        b.append(&table_cell(caption, true));
    }
    b
}

#[allow(dead_code)]
pub(super) fn table_row() -> gtk::Box {
    let b = row();
    b.set_spacing(0);
    b.add_css_class("table-row");
    b
}

#[allow(dead_code)]
pub(super) fn table_cell(text: &str, expand: bool) -> gtk::Label {
    let cell = label(text);
    cell.add_css_class("table-cell");
    cell.set_selectable(false);
    cell.set_hexpand(expand);
    cell.set_valign(gtk::Align::Center);
    cell
}

/// Keep mixed-script amounts and separators ordered, with stable tabular digits.
#[allow(dead_code)]
pub(super) fn numeric_cell(text: &str) -> gtk::Label {
    let cell = table_cell(text, false);
    cell.add_css_class("numeric");
    // Keep every monetary value readable on one line; the containing native
    // table scrolls when a long amount needs more room.
    cell.set_wrap(false);
    cell.set_direction(gtk::TextDirection::Ltr);
    cell.set_xalign(1.0);
    let attrs = gtk::pango::AttrList::new();
    attrs.insert(gtk::pango::AttrFontFeatures::new("tnum=1"));
    cell.set_attributes(Some(&attrs));
    cell
}

#[allow(dead_code)]
pub(super) fn status_label(text: &str, tone: StatusTone) -> gtk::Label {
    let status = label(text);
    status.add_css_class("status-label");
    status.add_css_class("caption");
    status.add_css_class(tone.class());
    if matches!(tone, StatusTone::Neutral) {
        status.add_css_class("dimmed");
    }
    status.set_selectable(false);
    status
}

#[allow(dead_code)]
pub(super) fn validation(message: &str, tone: StatusTone) -> gtk::Box {
    let b = column();
    b.add_css_class("validation");
    b.add_css_class("frame");
    b.add_css_class(tone.class());
    b.append(&label(message));
    b
}

#[allow(dead_code)]
pub(super) fn empty_state(title: &str, detail: &str) -> adw::StatusPage {
    let page = adw::StatusPage::builder()
        .title(title)
        .description(detail)
        .build();
    page.add_css_class("compact");
    page.add_css_class("empty-state");
    page
}
