mod app;

fn main() -> cosmic::iced::Result {
    let settings = cosmic::app::Settings::default().size(cosmic::iced::Size::new(1440.0, 940.0));

    cosmic::app::run::<app::App>(settings, ())
}
