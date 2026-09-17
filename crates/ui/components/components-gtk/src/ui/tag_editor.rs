use crate::{
    tr,
    ui::{SingleLineTextInput, control_row, tag_pill},
};
use adw::prelude::*;
use gtk::glib;
use std::{cell::RefCell, rc::Rc};

pub fn tag_editor(tags: Vec<String>, on_change: impl Fn(&[String]) + 'static) -> gtk::Box {
    let on_change = Rc::new(on_change);
    let tags = Rc::new(RefCell::new(tags));
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let pills = adw::WrapBox::builder()
        .child_spacing(6)
        .line_spacing(6)
        .build();
    populate(&pills, &tags, &on_change);
    let entry = SingleLineTextInput::builder("")
        .placeholder("Add tag")
        .build();
    entry.update_property(&[gtk::accessible::Property::Label(tr!("Add tag").as_ref())]);
    entry.connect_activate(glib::clone!(
        #[weak]
        pills,
        move |entry| {
            let tag = entry.text().trim().to_owned();
            if !tag.is_empty() && !tags.borrow().contains(&tag) {
                tags.borrow_mut().push(tag);
                on_change(&tags.borrow());
                populate(&pills, &tags, &on_change);
            }
            entry.set_text("");
        }
    ));
    content.append(&control_row("Tags", &entry));
    content.append(&pills);
    content
}

fn populate(
    pills: &adw::WrapBox,
    tags: &Rc<RefCell<Vec<String>>>,
    on_change: &Rc<impl Fn(&[String]) + 'static>,
) {
    while let Some(child) = pills.first_child() {
        pills.remove(&child);
    }
    pills.set_visible(!tags.borrow().is_empty());
    for tag in tags.borrow().iter() {
        let pill = tag_pill(tag, true);
        let remove_label = crate::i18n::text_args("Remove tag %{tag}", &[("tag", tag.clone())]);
        pill.update_property(&[gtk::accessible::Property::Label(&remove_label)]);
        pill.set_tooltip_text(Some(&remove_label));
        pill.connect_clicked(glib::clone!(
            #[weak]
            pills,
            #[strong]
            tags,
            #[strong]
            on_change,
            #[strong]
            tag,
            move |_| {
                tags.borrow_mut().retain(|existing| existing != &tag);
                on_change(&tags.borrow());
                populate(&pills, &tags, &on_change);
                pills
                    .parent()
                    .expect("tag pills remain in their control")
                    .child_focus(gtk::DirectionType::TabForward);
            }
        ));
        pills.append(&pill);
    }
}
