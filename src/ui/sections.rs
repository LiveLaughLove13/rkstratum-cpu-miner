use crate::ui::components::Components;
use crate::ui::theme::Theme;
use crate::AppState;
use egui::{RichText, TextEdit, Ui};

/// UI sections for the miner application
pub struct Sections;

impl Sections {
    /// Render the node connection section
    pub fn node_connection<F1, F2>(
        ui: &mut Ui,
        state: &mut AppState,
        on_connect: F1,
        on_disconnect: F2,
    ) where
        F1: FnOnce(),
        F2: FnOnce(),
    {
        Components::section_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Address:").color(Theme::LIGHT_GRAY));
                ui.add_space(10.0);
                ui.add(
                    TextEdit::singleline(&mut state.node_address)
                        .desired_width(400.0)
                        .frame(true),
                );
            });

            ui.add_space(15.0);

            ui.horizontal(|ui| {
                if state.is_connected {
                    if ui.add(Components::danger_button("🔌 Disconnect")).clicked() {
                        on_disconnect();
                    }
                } else {
                    if ui.add(Components::teal_button("⚡ Connect")).clicked() {
                        on_connect();
                    }
                }
            });
        });
    }

    /// Render the mining configuration section
    pub fn mining_config<F1, F2>(
        ui: &mut Ui,
        state: &mut AppState,
        num_cpus: usize,
        on_start: F1,
        on_stop: F2,
    ) where
        F1: FnOnce(),
        F2: FnOnce(),
    {
        Components::section_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Address:").color(Theme::LIGHT_GRAY));
                ui.add_space(10.0);
                ui.add(
                    TextEdit::singleline(&mut state.mining_address)
                        .desired_width(400.0)
                        .frame(true),
                );
            });

            ui.add_space(15.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("threads:").color(Theme::LIGHT_GRAY));
                ui.add_space(10.0);
                ui.add(egui::Slider::new(&mut state.threads, 1..=num_cpus).show_value(false));
                ui.label(RichText::new(format!("{}", state.threads)).color(Theme::LIGHT_GRAY));
            });

            ui.add_space(15.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Throttle (ms, optional):").color(Theme::LIGHT_GRAY));
                ui.add_space(10.0);
                let mut throttle_str = state.throttle_ms.map(|v| v.to_string()).unwrap_or_default();
                let response = ui.add(
                    TextEdit::singleline(&mut throttle_str)
                        .desired_width(150.0)
                        .frame(true),
                );
                if response.changed() {
                    state.throttle_ms = throttle_str.parse().ok();
                }
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        state.is_connected && !state.is_mining,
                        Components::primary_button("▶ Start Mining"),
                    )
                    .clicked()
                {
                    on_start();
                }

                ui.add_space(10.0);

                if ui
                    .add_enabled(state.is_mining, Components::danger_button("⏹ Stop Mining"))
                    .clicked()
                {
                    on_stop();
                }
            });
        });
    }

    /// Render the status section
    pub fn status(ui: &mut Ui, status_message: &str, status_type: &crate::StatusType) {
        Components::content_frame().show(ui, |ui| {
            if !status_message.is_empty() {
                let color = match status_type {
                    crate::StatusType::Success => Theme::PRIMARY_TEAL,
                    crate::StatusType::Error => Theme::RED,
                    crate::StatusType::Info => Theme::ACCENT_TEAL,
                };
                ui.label(RichText::new(status_message).color(color));
            } else {
                // Empty box
                ui.add_space(20.0);
            }
        });
    }

    /// Render the mining statistics section
    pub fn mining_stats(
        ui: &mut Ui,
        is_mining: bool,
        hashes: Option<u64>,
        blocks_submitted: Option<u64>,
        blocks_accepted: Option<u64>,
    ) {
        Components::content_frame().show(ui, |ui| {
            if is_mining {
                if let (Some(h), Some(bs), Some(ba)) = (hashes, blocks_submitted, blocks_accepted) {
                    ui.label(
                        RichText::new(format!("Hashes Tried: {}", h)).color(Theme::LIGHT_GRAY),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!("Blocks Submitted: {}", bs)).color(Theme::LIGHT_GRAY),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!("Blocks Accepted: {}", ba)).color(Theme::LIGHT_GRAY),
                    );
                } else {
                    ui.label(
                        RichText::new("Waiting for mining to start...").color(Theme::LIGHT_GRAY),
                    );
                }
            } else {
                ui.label(
                    RichText::new("Mining statistics will appear here when mining starts")
                        .color(Theme::LIGHT_GRAY),
                );
            }
        });
    }
}
