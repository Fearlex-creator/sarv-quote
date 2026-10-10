/// Opt in from a graphical session. Creates no application/window and does not
/// interact with the user's running desktop application.
#[test]
#[ignore = "requires graphical GTK/libadwaita initialization; creates no windows"]
fn native_widget_lifecycle_does_not_synthesize_new_or_empty_edit_actions() {
    use gtk::prelude::*;
    use std::{cell::Cell, rc::Rc};

    adw::init().unwrap();
    let clicked = Rc::new(Cell::new(0));
    let button = gtk::ToggleButton::with_label("آزمون ناوبری");
    let count = clicked.clone();
    button.connect_clicked(move |_| count.set(count.get() + 1));
    for _ in 0..10 {
        button.set_active(true);
        button.set_active(false);
    }
    assert_eq!(
        clicked.get(),
        0,
        "programmatic navigation activation clicked"
    );

    let changed = Rc::new(Cell::new(0));
    let entry = gtk::Entry::new();
    entry.set_text("P-101");
    let count = changed.clone();
    entry.connect_changed(move |_| count.set(count.get() + 1));
    let parent = gtk::Box::new(gtk::Orientation::Vertical, 0);
    parent.append(&entry);
    parent.remove(&entry);
    assert_eq!(entry.text(), "P-101");
    drop(entry);
    drop(parent);
    assert_eq!(
        changed.get(),
        0,
        "removal/disposal synthesized an empty edit"
    );
}
