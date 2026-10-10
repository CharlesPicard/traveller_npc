mod model;
mod names;
mod ui;

fn main() -> eframe::Result<()> {
    // winit 0.30 has no file drag-and-drop on Wayland, so prefer XWayland when it is
    // available. Set TRAVELLER_NPC_WAYLAND=1 to stay on native Wayland.
    if std::env::var_os("DISPLAY").is_some() && std::env::var_os("TRAVELLER_NPC_WAYLAND").is_none() {
        std::env::remove_var("WAYLAND_DISPLAY");
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1380.0, 1000.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Traveller NPC Generator"),
        ..Default::default()
    };
    eframe::run_native(
        "Traveller NPC Generator",
        options,
        Box::new(|cc| Ok(Box::new(ui::App::new(cc)))),
    )
}
