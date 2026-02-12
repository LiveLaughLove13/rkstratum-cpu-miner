use crate::ui::theme::Theme;
use egui::{Color32, Frame, RichText, Ui};

/// Reusable UI components
pub struct Components;

impl Components {
    /// Create a styled frame for sections
    pub fn section_frame() -> Frame {
        Frame::default()
            .fill(Theme::DARK_BG)
            .stroke(egui::Stroke::new(1.0, Theme::LIGHT_GRAY))
            .rounding(8.0)
            .inner_margin(egui::Margin::same(16.0))
    }

    /// Create a styled frame for content areas
    pub fn content_frame() -> Frame {
        Frame::default()
            .fill(Theme::DARK_BG)
            .stroke(egui::Stroke::new(1.0, Theme::LIGHT_GRAY))
            .rounding(6.0)
            .inner_margin(egui::Margin::same(12.0))
    }

    /// Render a section header with icon, chevron, title and subtitle
    pub fn section_header(
        ui: &mut Ui,
        icon: &str,
        title: &str,
        subtitle: &str,
        is_open: bool,
    ) -> bool {
        let mut clicked = false;
        ui.horizontal(|ui| {
            // Square icon
            ui.label(RichText::new(icon).size(12.0).color(Theme::LIGHT_GRAY));
            ui.add_space(8.0);

            // Chevron
            let chevron = if is_open { "▼" } else { "▶" };
            if ui.selectable_label(false, chevron).clicked() {
                clicked = true;
            }
            ui.add_space(8.0);

            // Title
            ui.label(RichText::new(title).size(18.0).color(Theme::WHITE));
            ui.add_space(8.0);

            // Subtitle
            ui.label(RichText::new(subtitle).size(13.0).color(Theme::LIGHT_GRAY));
        });
        clicked
    }

    /// Render a status indicator dot with text
    pub fn status_indicator(ui: &mut Ui, color: Color32, text: &str) {
        ui.horizontal(|ui| {
            let dot_pos = ui.available_rect_before_wrap().min + egui::vec2(6.0, 8.0);
            ui.painter().circle_filled(dot_pos, 6.0, color);
            ui.add_space(12.0);
            ui.label(RichText::new(text).color(Theme::LIGHT_GRAY).size(13.0));
        });
    }

    /// Create a styled button with teal background
    pub fn teal_button(text: &str) -> egui::Button {
        egui::Button::new(RichText::new(text).color(Theme::WHITE))
            .fill(Theme::ACCENT_TEAL)
            .rounding(6.0)
            .min_size(egui::vec2(150.0, 35.0))
    }

    /// Create a styled button with primary teal background
    pub fn primary_button(text: &str) -> egui::Button {
        egui::Button::new(RichText::new(text).color(Theme::WHITE))
            .fill(Theme::PRIMARY_TEAL)
            .rounding(6.0)
            .min_size(egui::vec2(150.0, 35.0))
    }

    /// Create a styled button with red background
    pub fn danger_button(text: &str) -> egui::Button {
        egui::Button::new(RichText::new(text).color(Theme::WHITE))
            .fill(Theme::RED)
            .rounding(6.0)
            .min_size(egui::vec2(150.0, 35.0))
    }
}
