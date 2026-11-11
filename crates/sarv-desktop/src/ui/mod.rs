//! Root view: rune-kit `SplitView` shell, active screen, dialog/toast overlays.

pub mod documents;
pub mod glass;
pub mod icons;
pub mod kit;
pub mod ops;
pub mod pickers;
pub mod resources;
pub mod shell;
pub mod views;

use rune::{Cx, Direction, El, col};

use crate::app::{Page, State};
use crate::ui::glass::Ui;

pub fn root_view(s: &State, cx: &mut Cx<State>) -> El {
    let ui = Ui::new(s);
    let dir = if s.lang().is_rtl() { Direction::Rtl } else { Direction::Ltr };
    let mut layer = Vec::new();

    let content = if !s.books.has_company() || !s.books.initialized() {
        views::setup::view(s, cx, &ui, &mut layer)
    } else {
        match s.page {
            Page::Overview => views::overview::view(s, cx, &ui, &mut layer),
            Page::Sales => views::sales::view(s, cx, &ui, &mut layer),
            Page::Pos => views::pos::view(s, cx, &ui, &mut layer),
            Page::Purchases => views::purchases::view(s, cx, &ui, &mut layer),
            Page::Payments => views::payments::view(s, cx, &ui, &mut layer),
            Page::Banking => views::banking::view(s, cx, &ui, &mut layer),
            Page::Contacts => views::contacts::view(s, cx, &ui, &mut layer),
            Page::Inventory => views::inventory::view(s, cx, &ui, &mut layer),
            Page::Accounting => views::accounting::view(s, cx, &ui, &mut layer),
            Page::Reports => views::reports::view(s, cx, &ui, &mut layer),
            Page::Sheets => views::sheets::view(s, cx, &ui, &mut layer),
            Page::Settings => views::settings::view(s, cx, &ui, &mut layer),
        }
    };

    shell::mount_palette(s, &ui, &mut layer);
    shell::mount_toasts(s, &ui, &mut layer);

    let ready = s.books.has_company() && s.books.initialized();
    let window = if ready {
        shell::app_shell(s, &ui, cx, content)
    } else {
        col()
            .key("setup-window")
            .w_full()
            .h_full()
            .fill(ui.k.c.page)
            .child(content.grow(1.0).min_h(0.0))
    };
    rune_kit::root(window, layer).key("root").dir(dir).fill(ui.k.c.page)
}
