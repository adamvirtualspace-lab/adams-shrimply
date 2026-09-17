use gtk::prelude::*;
use std::cell::Cell;

const MAX_TAG_LABEL_CHARS: i32 = 24;

thread_local! {
    static CSS_INSTALLED: Cell<bool> = const { Cell::new(false) };
}

pub fn tag_pill(tag: &str, removable: bool) -> gtk::Button {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.append(
        &gtk::Label::builder()
            .label(tag)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(MAX_TAG_LABEL_CHARS)
            .build(),
    );
    if removable {
        content.append(&gtk::Image::from_icon_name("window-close-symbolic"));
    }
    let pill = gtk::Button::builder()
        .child(&content)
        .css_classes(["pill", "tag-pill"])
        .tooltip_text(tag)
        .focusable(removable)
        .can_target(removable)
        .build();
    CSS_INSTALLED.with(|installed| {
        if installed.replace(true) {
            return;
        }
        let provider = gtk::CssProvider::new();
        provider.load_from_string(
            "button.tag-pill { min-height: 20px; min-width: 0; padding: 4px 12px; font-weight: normal; }",
        );
        gtk::style_context_add_provider_for_display(
            &pill.display(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
    pill
}
