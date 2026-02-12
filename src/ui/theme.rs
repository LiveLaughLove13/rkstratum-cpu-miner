use egui::Color32;

/// Color palette for the Kaspa CPU Miner GUI
pub struct Theme;

impl Theme {
    // Primary colors from hex codes
    pub const PRIMARY_TEAL: Color32 = Color32::from_rgb(112, 199, 186); // #70C7BA
    pub const DARK_BG: Color32 = Color32::from_rgb(35, 31, 32); // #231F20
    pub const LIGHT_GRAY: Color32 = Color32::from_rgb(182, 182, 182); // #B6B6B6
    pub const ACCENT_TEAL: Color32 = Color32::from_rgb(73, 234, 203); // #49EACB

    // Additional colors
    pub const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
    pub const RED: Color32 = Color32::from_rgb(220, 53, 69);
    pub const GREEN: Color32 = Color32::from_rgb(34, 197, 94);

    /// Apply the theme to egui visuals
    pub fn apply(visuals: &mut egui::style::Visuals) {
        visuals.dark_mode = true;
        visuals.panel_fill = Self::DARK_BG;
        visuals.window_fill = Self::DARK_BG;
        visuals.faint_bg_color = Self::DARK_BG;
        visuals.extreme_bg_color = Self::DARK_BG;
        visuals.code_bg_color = Self::DARK_BG;
        visuals.warn_fg_color = Self::ACCENT_TEAL;
        visuals.error_fg_color = Self::RED;
        visuals.override_text_color = Some(Self::LIGHT_GRAY);
        visuals.selection.bg_fill = Self::PRIMARY_TEAL;
        visuals.selection.stroke.color = Self::PRIMARY_TEAL;

        // Widget states
        visuals.widgets.inactive.bg_fill = Self::DARK_BG;
        visuals.widgets.inactive.bg_stroke.color = Self::LIGHT_GRAY;
        visuals.widgets.inactive.weak_bg_fill = Self::DARK_BG;

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 41, 42);
        visuals.widgets.hovered.bg_stroke.color = Self::PRIMARY_TEAL;

        visuals.widgets.active.bg_fill = Self::PRIMARY_TEAL;
        visuals.widgets.active.bg_stroke.color = Self::PRIMARY_TEAL;

        visuals.widgets.open.bg_fill = Self::DARK_BG;
        visuals.widgets.open.bg_stroke.color = Self::PRIMARY_TEAL;
    }
}
