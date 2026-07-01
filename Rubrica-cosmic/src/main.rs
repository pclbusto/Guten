mod app;
mod document;
mod settings;
mod ui;

fn main() -> cosmic::iced::Result {
    let settings = cosmic::app::Settings::default().size(cosmic::iced::Size::new(1200.0, 800.0));
    let project_path = std::env::args_os().nth(1).map(std::path::PathBuf::from);

    cosmic::app::run::<app::App>(settings, project_path)
}
