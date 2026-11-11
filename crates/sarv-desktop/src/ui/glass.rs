//! Thin helpers over rune-kit defaults (Button, Card, Dialog, Field, Badge, Toast chrome).

use rune::{Color, El, Icon, Transition, col, el, icon, row, text};
use rune_kit::{
    Badge, BadgeVariant, Button, ButtonVariant, Card, ControlSize, Divider, Field, IconButton,
    IconButtonVariant, Kit, SegmentedControl, TextInput,
};

use crate::app::{State, Tone};
use crate::i18n::Fmt;
use crate::ui::kit::sarv_kit;

#[derive(Clone)]
pub struct Ui {
    pub k: Kit,
    pub f: Fmt,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    Primary,
    Glass,
    Ghost,
    Danger,
}

impl Ui {
    pub fn new(s: &State) -> Ui {
        Ui { k: sarv_kit(s.dark), f: s.fmt }
    }

    pub fn t(&self, key: &str) -> String {
        self.f.t(key)
    }

    pub fn tone_fg(&self, tone: Tone) -> Color {
        match tone {
            Tone::Success => self.k.c.success.fg,
            Tone::Warning => self.k.c.warning.fg,
            Tone::Danger => self.k.c.danger.fg,
            Tone::Info => self.k.c.info.fg,
        }
    }

    pub fn tone_accent(&self, tone: Tone) -> Color {
        match tone {
            Tone::Success => self.k.c.success.icon,
            Tone::Warning => self.k.c.warning.icon,
            Tone::Danger => self.k.c.danger.icon,
            Tone::Info => self.k.c.info.icon,
        }
    }

    pub fn body(&self, s: impl Into<String>) -> El {
        text(s.into()).font(self.k.ty.ui).color(self.k.c.text_base)
    }

    pub fn muted(&self, s: impl Into<String>) -> El {
        text(s.into()).font(self.k.ty.caption()).color(self.k.c.text_muted)
    }

    pub fn heading(&self, s: impl Into<String>) -> El {
        text(s.into()).font(self.k.ty.ui_medium).color(self.k.c.text_base)
    }

    pub fn title(&self, s: impl Into<String>) -> El {
        text(s.into()).font(self.k.ty.body_medium).color(self.k.c.text_base)
    }

    pub fn ic(&self, i: &'static Icon, size: f32, color: Color) -> El {
        icon(i, size).color(color)
    }

    pub fn button(&self, kind: Btn, id: &str, label: &str, glyph: Option<&'static Icon>, on_press: impl Fn(&mut State) + 'static) -> El {
        let variant = match kind {
            Btn::Primary => ButtonVariant::Submit,
            Btn::Danger => ButtonVariant::Danger,
            Btn::Glass => ButtonVariant::Outline,
            Btn::Ghost => ButtonVariant::Ghost,
        };
        let mut b = Button::new(label.to_string()).variant(variant).control_size(ControlSize::Large).on_press(on_press);
        if let Some(g) = glyph {
            b = b.icon(g);
        }
        b.build(&self.k).key(format!("btn-{id}"))
    }

    /// A toolbar action that shares a row and wraps onto the next line in a narrow window.
    pub fn wrap_btn(&self, kind: Btn, key: &str, glyph: Option<&'static Icon>, on_press: impl Fn(&mut State) + 'static) -> El {
        let variant = match kind {
            Btn::Primary => ButtonVariant::Submit,
            Btn::Danger => ButtonVariant::Danger,
            Btn::Glass => ButtonVariant::Outline,
            Btn::Ghost => ButtonVariant::Ghost,
        };
        let label = self.t(key);
        let mut b = Button::new(label).variant(variant).control_size(ControlSize::Small).on_press(on_press);
        if let Some(g) = glyph {
            b = b.icon(g);
        }
        b.build(&self.k).key(format!("btn-{key}"))
    }

    /// A short action for a list row. It stays as wide as its label and wraps with its siblings.
    pub fn compact_btn(&self, kind: Btn, key: &str, on_press: impl Fn(&mut State) + 'static) -> El {
        let variant = match kind {
            Btn::Primary => ButtonVariant::Submit,
            Btn::Danger => ButtonVariant::Danger,
            Btn::Glass => ButtonVariant::Outline,
            Btn::Ghost => ButtonVariant::Ghost,
        };
        Button::new(self.t(key))
            .variant(variant)
            .control_size(ControlSize::Small)
            .on_press(on_press)
            .build(&self.k)
            .key(format!("btn-{key}"))
    }

    /// A small text button with its own label, for a count that navigates.
    pub fn text_btn(&self, id: &str, label: &str, on_press: impl Fn(&mut State) + 'static) -> El {
        Button::new(label.to_string())
            .variant(ButtonVariant::Ghost)
            .control_size(ControlSize::Small)
            .on_press(on_press)
            .build(&self.k)
            .key(format!("btn-{id}"))
    }

    /// Frosted sheet. macOS uses system glass; every platform also blurs what is already drawn behind it.
    pub fn frost(&self, key: &str, body: El) -> El {
        let tint = self.k.c.layer_01.alpha(0.72);
        col()
            .key(key.to_string())
            .w_full()
            .radius(14.0)
            .system_material(rune::SystemMaterial::Glass)
            .backdrop_blur(18.0)
            .fill(tint)
            .border(0.5, self.k.c.border_base)
            .px(14.0)
            .py(12.0)
            .child(body)
    }

    pub fn btn(&self, kind: Btn, key: &str, glyph: Option<&'static Icon>, on_press: impl Fn(&mut State) + 'static) -> El {
        let label = self.t(key);
        self.button(kind, key, &label, glyph, on_press)
    }

    pub fn field(&self, s: &State, key: &str, placeholder: &str) -> El {
        let k = key.to_string();
        let edit = s.forms.view(key);
        TextInput::new(&edit)
            .placeholder(placeholder.to_string())
            .on_edit(move |st: &mut State| st.forms.edit(&k))
            .build(&self.k)
            .key(format!("field-{key}"))
            .w_full()
    }

    pub fn labeled(&self, label_key: &str, control: El) -> El {
        Field::new().label(self.t(label_key)).control(control).build(&self.k)
    }

    pub fn badge(&self, tone: Tone, s: impl Into<String>) -> El {
        let variant = match tone {
            Tone::Info => BadgeVariant::Accent,
            _ => BadgeVariant::Subtle,
        };
        Badge::new(s.into()).variant(variant).build(&self.k).self_start()
    }

    pub fn tabs(&self, id: &str, items: &[(&str, bool)], on_pick: impl Fn(&mut State, usize) + Clone + 'static) -> El {
        let labels: Vec<String> = items.iter().map(|(l, _)| (*l).to_string()).collect();
        let selected = items.iter().position(|(_, sel)| *sel);
        SegmentedControl::new(&labels, selected)
            .fit()
            .label(id.to_string())
            .on_pick(move |st: &mut State, i, _| on_pick(st, i))
            .build(&self.k)
            .key(id.to_string())
    }

    pub fn card(&self) -> El {
        // Kit Card paints the accent rule; callers add children via `.child`.
        Card::new().build(&self.k)
    }

    pub fn kit_card(&self) -> Card {
        Card::new()
    }

    pub fn section(&self, key: &str, title: &str, body: El) -> El {
        col()
            .key(key.to_string())
            .gap(8.0)
            .child(text(title.to_string()).font(self.k.ty.text_xsmall_medium).color(self.k.c.text_muted).ml(8.0))
            .child(body)
    }

    pub fn panel_rows(&self, key: &str, rows: Vec<El>) -> El {
        let n = rows.len();
        let mut body = col().key("body").w_full().padding(4.0);
        for (i, r) in rows.into_iter().enumerate() {
            body = body.child(r);
            if i + 1 < n {
                body = body.child(Divider::horizontal().build(&self.k).ml(12.0).mr(12.0));
            }
        }
        self.k
            .elevate(
                col()
                    .key(key.to_string())
                    .w_full()
                    .radius(self.k.radius.md)
                    .fill(self.k.c.layer_01)
                    .border(0.5, self.k.c.border_base)
                    .child(body),
                &self.k.elevation.raised,
            )
    }

    /// The page's primary action stays in the toolbar. An empty list shows that instead of a blank table.
    pub fn fill_or_empty(&self, count: usize, title: &str, detail: &str, glyph: &'static Icon, table: El) -> El {
        if count == 0 { self.empty(glyph, title, detail).grow(1.0).min_h(0.0) } else { table }
    }

    pub fn empty(&self, glyph: &'static Icon, title: &str, detail: &str) -> El {
        Card::new()
            .title(self.t(title), Some(glyph))
            .description(self.t(detail))
            .build(&self.k)
            .w_full()
    }

    pub fn metric(&self, key: &str, glyph: &'static Icon, title: &str, value: String, sub: String, tone: Tone) -> El {
        Card::new()
            .accent(self.tone_accent(tone))
            .title(title.to_string(), Some(glyph))
            .description(sub)
            .child(text(value).font(self.k.ty.body_medium).color(self.k.c.text_base))
            .build(&self.k)
            .key(key.to_string())
            .grow(1.0)
    }

    /// Document page inside the shell panel.
    ///
    /// The body fills the space under the heading (`grow` + `min_h(0)`) so a kit `Table` can
    /// be the scroll viewport. Wrap a long column yourself with `.scroll(..)` before passing it.
    pub fn page(&self, key: &str, title: &str, subtitle: &str, actions: Vec<El>, body: El) -> El {
        // The shell toolbar is the document title. This column keeps that name for accessibility.
        let page = col()
            .key(key.to_string())
            .label(title.to_string())
            .grow(1.0)
            .h_full()
            .min_h(0.0)
            .gap(16.0)
            .px(24.0)
            .pt(8.0)
            .pb(16.0);
        let mut titles = col().key("page-titles").gap(2.0).grow(1.0).min_w(0.0);
        if !subtitle.is_empty() {
            titles = titles.child(self.muted(subtitle.to_string()));
        }
        let mut head = row().key("page-head").w_full().gap(12.0).center_y().child(titles);
        head = actions.into_iter().fold(head, |h, a| h.child(a));
        page.child(head).child(body.grow(1.0).min_h(0.0)).transition(Transition::opacity())
    }

    pub fn cell(&self, s: impl Into<String>) -> El {
        text(s.into()).font(self.k.ty.ui).color(self.k.c.text_base).truncate()
    }

    pub fn cell_muted(&self, s: impl Into<String>) -> El {
        text(s.into()).font(self.k.ty.ui).color(self.k.c.text_muted).truncate()
    }

    /// Mount a kit `Dialog` into the overlay layer when `open`.
    pub fn dialog(
        &self,
        layer: &mut Vec<El>,
        key: &str,
        title: &str,
        open: bool,
        width: f32,
        body: El,
        footer: Vec<El>,
        on_close: impl Fn(&mut State) + Clone + 'static,
    ) {
        self.dialog_described(layer, &mut Vec::new(), key, title, "", open, width, body, footer, on_close);
    }

    /// Dialog with an optional muted description under the title.
    ///
    /// Pass picker portals in `portals` (build Combobox/Select into that vec). They are appended
    /// after the dialog so menus paint above the panel, anchored under their fields.
    pub fn dialog_described(
        &self,
        layer: &mut Vec<El>,
        portals: &mut Vec<El>,
        key: &str,
        title: &str,
        description: &str,
        open: bool,
        width: f32,
        body: El,
        footer: Vec<El>,
        on_close: impl Fn(&mut State) + Clone + 'static,
    ) {
        if !open {
            return;
        }
        let k = &self.k;
        let close = on_close.clone();
        let dismiss = IconButton::new(k.glyph.close)
            .variant(IconButtonVariant::Ghost)
            .control_size(ControlSize::Regular)
            .label(self.t("Close"))
            .build(k)
            .key(format!("{key}-close"))
            .on_press(close);
        let mut heading = col()
            .grow(1.0)
            .min_w(0.0)
            .gap(4.0)
            .child(text(title.to_string()).font(k.ty.ui_medium).color(k.c.text_base));
        if !description.is_empty() {
            heading = heading.child(text(description.to_string()).font(k.ty.ui).color(k.c.text_muted));
        }
        let scrolled = body.scroll(rune::ScrollSpec { stick_bottom: false }).w_full().max_h(420.0);
        let mut foot = row().key("dialog-footer").w_full().gap(8.0).px(16.0).py(12.0).center_y();
        if !footer.is_empty() {
            foot = foot.child(el().grow(1.0));
        }
        let foot = footer.into_iter().fold(foot, |r, b| r.child(b));
        let mut panel = col()
            .key(format!("{key}-dialog"))
            .role("dialog")
            .label(title.to_string())
            .description(description.to_string())
            .w_full()
            .max_w(width)
            .radius(14.0)
            .fill(k.c.layer_01)
            .border(0.5, k.c.border_base)
            .child(
                row()
                    .key("header")
                    .w_full()
                    .padding(16.0)
                    .gap(12.0)
                    .center_y()
                    .child(heading)
                    .child(dismiss),
            )
            .child(col().key("dialog-body").w_full().px(16.0).pb(4.0).child(scrolled))
            .child(el().w_full().h(0.5).fill(k.c.border_base))
            .child(foot)
            .token_motion(k.motion.overlay)
            .modal(true, on_close);
        for shadow in k.shadow_layers(&k.elevation.overlay) {
            panel = panel.box_shadow(shadow);
        }
        layer.push(
            el().key(format!("{key}-scrim"))
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .fill(k.c.scrim)
                .token_motion(k.motion.overlay)
                .a11y_hidden(),
        );
        layer.push(
            el().key(format!("{key}-layer"))
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .padding(16.0)
                .center()
                .child(panel),
        );
        // Menus are built against their fields, then mounted after the sheet so they paint on top
        // of it and still anchor to those fields.
        layer.append(portals);
    }

    /// Compact two-column field row for facture editors.
    pub fn form_grid(&self, left: El, right: El) -> El {
        row().w_full().gap(12.0).child(left.grow(1.0).min_w(0.0)).child(right.grow(1.0).min_w(0.0))
    }

}
