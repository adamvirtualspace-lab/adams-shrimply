use shrimply_components_gtk::ui::MultilineTextInput;
use shrimply_project_document::project::{CommentColor, CommentItem};

use super::{Inspectable, InspectorContext, section::InspectorSection, selector::selector};
use crate::player_state::{self, ProjectChange};

impl Inspectable for CommentItem {
    fn title(&self) -> &'static str {
        "Comment"
    }

    fn add_rows(&self, section: &InspectorSection, context: &InspectorContext) {
        let Some(key) = context.selected_item.clone() else {
            return;
        };
        let project = context.project.clone();
        let player = context.player_state.clone();
        let commit_project = project.clone();
        let text_key = key.clone();
        let editor = MultilineTextInput::builder(&self.text)
            .on_change(move |text| {
                let mut project = project.borrow_mut();
                let Some(item) = project.comment_item_mut(&text_key) else {
                    return false;
                };
                if item.text == text {
                    return false;
                }
                item.text = text;
                drop(project);
                player_state::refresh_project(&player, ProjectChange::default());
                true
            })
            .on_commit(move || {
                shrimply_project_document::project::commit_edit(
                    &commit_project.borrow(),
                    "comment",
                );
            })
            .build();
        section.add_control_row("Content", editor.widget());
        let project = context.project.clone();
        let player = context.player_state.clone();
        let dropdown = selector(
            "Color",
            self.color,
            CommentColor::ALL
                .into_iter()
                .map(|color| (color, color.label())),
            move |color| {
                let mut project = project.borrow_mut();
                let Some(item) = project.comment_item_mut(&key) else {
                    return;
                };
                if item.color == color {
                    return;
                }
                item.color = color;
                shrimply_project_document::project::commit_edit(&project, "comment-color");
                drop(project);
                player_state::refresh_project(&player, ProjectChange::default());
            },
        );
        section.add_wide_control(&dropdown);
    }
}
