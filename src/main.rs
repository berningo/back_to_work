mod model;
mod storage;
mod ui;

use model::Planner;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([900.0, 680.0])
            .with_min_inner_size([620.0, 460.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Aufgabenplaner",
        options,
        Box::new(|_cc| {
            let path = storage::database_path();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| eframe::Error::AppCreation(Box::new(error)))?;
            }
            let db = storage::open(&path)
                .map_err(|error| eframe::Error::AppCreation(Box::new(error)))?;
            Ok(Box::new(Planner::from_database(db)))
        }),
    )
}
