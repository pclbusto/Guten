use cosmic::app::{Core, Task};
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, container, text};
use cosmic::{Application, Element};

use visore::viewer::{self, ViewerMessage};
use visore::{dark_bg, ViewerState};

use rfd::AsyncFileDialog;

fn perform_async<T: Send + 'static>(
    future: impl std::future::Future<Output = T> + Send + 'static,
    f: impl FnOnce(T) -> AppMessage + Send + 'static,
) -> Task<AppMessage> {
    cosmic::task::future(async move { f(future.await) })
}

#[derive(Debug, Clone)]
enum AppMessage {
    Viewer(ViewerMessage),
    OpenFile,
    FileSelected(Option<std::path::PathBuf>),
}

struct App {
    core: Core,
    state: ViewerState,
    show_save_dropdown: bool,
}

impl Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = AppMessage;
    const APP_ID: &'static str = "com.guten.Visore";

    fn core(&self) -> &Core { &self.core }
    fn core_mut(&mut self) -> &mut Core { &mut self.core }

    fn init(core: Core, _flags: ()) -> (Self, Task<AppMessage>) {
        let state = ViewerState::default();
        (App { core, state, show_save_dropdown: false }, Task::none())
    }

    fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::OpenFile => {
                return perform_async(
                    async {
                        AsyncFileDialog::new()
                            .add_filter("Imagen", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    AppMessage::FileSelected,
                );
            }
            AppMessage::FileSelected(Some(path)) => {
                let _ = self.state.load_image(&path);
            }
            AppMessage::FileSelected(None) => {}
            AppMessage::Viewer(msg) => {
                match msg {
                    ViewerMessage::Cancel => {
                        std::process::exit(0);
                    }
                    ViewerMessage::ToggleSaveDropdown => {
                        self.show_save_dropdown = !self.show_save_dropdown;
                    }
                    ViewerMessage::Save => {}
                    ViewerMessage::SaveAs(format) => {
                        if let Some(ref img) = self.state.image_original {
                            let _ = img.save(format!("/tmp/visore_output.{}", format));
                        }
                    }
                    ViewerMessage::SetTheme(t) => {
                        self.state.theme = t;
                    }
                    ViewerMessage::SetAspectRatio(ratio) => {
                        self.state.aspect_ratio = ratio;
                    }
                    ViewerMessage::SetOrientation(ori) => {
                        self.state.orientation = ori;
                    }
                    ViewerMessage::RotateCW => self.state.rotate_cw(),
                    ViewerMessage::RotateCCW => self.state.rotate_ccw(),
                    ViewerMessage::FlipH => self.state.toggle_flip_h(),
                    ViewerMessage::FlipV => self.state.toggle_flip_v(),
                    ViewerMessage::CropChanged(rect) => {
                        self.state.crop_rect = rect;
                    }
                    _ => {}
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, AppMessage> {
        let open_btn = if self.state.image_path.is_none() {
            button::standard("Abrir imagen").on_press(AppMessage::OpenFile)
        } else {
            button::standard("Abrir otra").on_press(AppMessage::OpenFile)
        };

        let viewer_content = viewer::viewer_view(&self.state, self.show_save_dropdown)
            .map(AppMessage::Viewer);

        if self.state.image_path.is_none() {
            container(
                column!(
                    cosmic::widget::Space::new().height(Length::Fill),
                    text::body("Visore - Image Viewer"),
                    open_btn,
                    cosmic::widget::Space::new().height(Length::Fill),
                )
                .align_x(Alignment::Center)
                .spacing(20),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &cosmic::Theme| cosmic::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(dark_bg())),
                ..Default::default()
            })
            .into()
        } else {
            viewer_content
        }
    }
}

fn main() -> cosmic::iced::Result {
    let settings = cosmic::app::Settings::default()
        .size(cosmic::iced::Size::new(1200.0, 800.0));

    cosmic::app::run::<App>(settings, ())
}
