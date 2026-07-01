use cosmic::iced::{Color, Length};
use cosmic::widget;
use cosmic::Element;

use crate::{AspectRatio, CropRect};

#[derive(Debug, Clone)]
pub enum CropMessage {
    DragStart,
}

pub struct CropView {
    image: widget::image::Handle,
    _crop: CropRect,
    _aspect: AspectRatio,
}

impl CropView {
    pub fn new(image: widget::image::Handle, crop: CropRect, aspect: AspectRatio) -> Self {
        Self { image, _crop: crop, _aspect: aspect }
    }

    pub fn view<'a>(&self) -> Element<'a, CropMessage> {
        let img = widget::image(self.image.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(cosmic::iced::ContentFit::Contain);

        widget::container(img)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &cosmic::Theme| widget::container::Style {
                background: Some(cosmic::iced::Background::Color(Color::from_rgba8(18, 18, 22, 1.0))),
                ..Default::default()
            })
            .into()
    }
}
