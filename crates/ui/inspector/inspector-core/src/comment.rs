use shrimply_project_document::project::{CommentColor, CommentItem};

use crate::{ControlKind, InspectorControl, InspectorSection};

pub fn section(item: &CommentItem) -> InspectorSection {
    let mut section = InspectorSection::default();
    section.add(
        InspectorControl::new(ControlKind::MultilineText, "/text", "Content").value(&item.text),
    );
    section.add(
        InspectorControl::new(ControlKind::Selector, "/color", "Color")
            .value(item.color.label().to_lowercase())
            .choices(
                CommentColor::ALL
                    .iter()
                    .map(|color| color.label().to_lowercase())
                    .collect(),
                CommentColor::ALL
                    .iter()
                    .map(|color| color.label().to_string())
                    .collect(),
            ),
    );
    section
}
