use adw::prelude::*;
use gtk::glib;
use shrimply_components_gtk::{
    tr,
    ui::{control_row, tag_pill},
};
use shrimply_inspector_core::InspectorController;
use std::{cell::RefCell, rc::Rc};

pub(super) fn control(tags: Vec<String>, controller: InspectorController) -> gtk::Box {
    let tags = Rc::new(RefCell::new(tags));
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let pills = adw::WrapBox::builder()
        .child_spacing(6)
        .line_spacing(6)
        .build();
    populate(&pills, &tags, &controller);
    let entry = gtk::Entry::builder()
        .placeholder_text(tr!("Add tag").as_ref())
        .hexpand(true)
        .build();
    entry.update_property(&[gtk::accessible::Property::Label(tr!("Add tag").as_ref())]);
    entry.connect_activate(glib::clone!(
        #[weak]
        pills,
        move |entry| {
            let tag = entry.text().trim().to_owned();
            if !tag.is_empty() && !tags.borrow().contains(&tag) {
                tags.borrow_mut().push(tag);
                controller.set_project_tags(&tags.borrow());
                populate(&pills, &tags, &controller);
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
    controller: &InspectorController,
) {
    while let Some(child) = pills.first_child() {
        pills.remove(&child);
    }
    pills.set_visible(!tags.borrow().is_empty());
    for tag in tags.borrow().iter() {
        let pill = tag_pill(tag, true);
        let remove_label =
            shrimply_components_gtk::i18n::text_args("Remove tag %{tag}", &[("tag", tag.clone())]);
        pill.update_property(&[gtk::accessible::Property::Label(&remove_label)]);
        pill.set_tooltip_text(Some(&remove_label));
        pill.connect_clicked(glib::clone!(
            #[weak]
            pills,
            #[strong]
            tags,
            #[strong]
            controller,
            #[strong]
            tag,
            move |_| {
                tags.borrow_mut().retain(|existing| existing != &tag);
                controller.set_project_tags(&tags.borrow());
                populate(&pills, &tags, &controller);
                pills
                    .parent()
                    .expect("tag pills remain in their control")
                    .child_focus(gtk::DirectionType::TabForward);
            }
        ));
        pills.append(&pill);
    }
}
