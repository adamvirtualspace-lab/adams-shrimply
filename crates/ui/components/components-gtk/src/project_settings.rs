use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use shrimply_components_core::project_settings::{CUSTOM_PRESET_INDEX, ProjectSettings};
use shrimply_project_types::{
    COMMON_FRAME_RATES, MAX_CANVAS_DIMENSION, MIN_CANVAS_DIMENSION, PROJECT_PRESETS,
};

use crate::ui::{Number2Picker, control_row, dropdown};

pub struct ProjectSettingsSelector {
    pub group: adw::PreferencesGroup,
    model: Rc<Cell<ProjectSettings>>,
}

impl ProjectSettingsSelector {
    pub fn new() -> Self {
        let initial = ProjectSettings::default();
        let model = Rc::new(Cell::new(initial));
        let updating = Rc::new(Cell::new(false));
        let preset = dropdown(
            initial.preset,
            PROJECT_PRESETS
                .iter()
                .map(|preset| preset.label)
                .chain(["Custom"])
                .enumerate(),
            {
                let model = model.clone();
                move |index| {
                    let mut settings = model.get();
                    settings.select_preset(index);
                    model.set(settings);
                }
            },
        );
        let resolution =
            Number2Picker::builder(f64::from(initial.width), f64::from(initial.height))
                .minimum(f64::from(MIN_CANVAS_DIMENSION))
                .maximum(f64::from(MAX_CANVAS_DIMENSION))
                .drag_step(1.0)
                .digits(0)
                .width_chars(7)
                .first_prefix("W")
                .second_prefix("H")
                .enable_lock()
                .on_change({
                    let preset = preset.clone();
                    let model = model.clone();
                    move |values, _| {
                        let mut settings = model.get();
                        settings.set_width(values[0].round() as u32);
                        settings.set_height(values[1].round() as u32);
                        model.set(settings);
                        preset.set_selected(CUSTOM_PRESET_INDEX as u32);
                    }
                })
                .build_with_handles();
        let fps = dropdown(
            initial.frame_rate,
            COMMON_FRAME_RATES.iter().map(|rate| rate.label).enumerate(),
            {
                let preset = preset.clone();
                let updating = updating.clone();
                let model = model.clone();
                move |index| {
                    if updating.get() {
                        return;
                    }
                    let mut settings = model.get();
                    settings.set_frame_rate(index);
                    model.set(settings);
                    preset.set_selected(CUSTOM_PRESET_INDEX as u32);
                }
            },
        );
        preset.connect_selected_notify({
            let width = resolution.first.downgrade();
            let height = resolution.second.downgrade();
            let fps = fps.downgrade();
            let model = model.clone();
            move |preset| {
                if preset.selected() as usize == CUSTOM_PRESET_INDEX {
                    return;
                }
                let settings = model.get();
                updating.set(true);
                if let Some(width) = width.upgrade() {
                    width.set_f64(f64::from(settings.width));
                }
                if let Some(height) = height.upgrade() {
                    height.set_f64(f64::from(settings.height));
                }
                if let Some(fps) = fps.upgrade() {
                    fps.set_selected(settings.frame_rate as u32);
                }
                updating.set(false);
            }
        });
        let controls = gtk::Box::new(gtk::Orientation::Vertical, 12);
        controls.append(&control_row("Preset", &preset));
        controls.append(&control_row("Resolution", &resolution.widget));
        controls.append(&control_row("Frame Rate", &fps));
        let settings = adw::PreferencesGroup::builder()
            .title(crate::i18n::text("Project Settings").as_ref())
            .build();
        settings.add(&controls);
        Self {
            group: settings,
            model,
        }
    }

    pub fn settings(
        &self,
    ) -> Option<(
        shrimply_project_types::CanvasSize,
        shrimply_math_core::Fraction,
    )> {
        self.model.get().settings()
    }
}

impl Default for ProjectSettingsSelector {
    fn default() -> Self {
        Self::new()
    }
}
