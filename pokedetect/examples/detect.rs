use iced::widget::{bottom_right, container, image, stack, text};
use iced::{Element, Fill, Font, Subscription};

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application(Detect::new, Detect::update, Detect::view)
        .subscription(Detect::subscription)
        .run()
}

enum Detect {
    Loading,
    Capturing {
        image: image::Handle,
        detection: Option<Detection>,
    },
}

#[derive(Debug, Clone)]
struct Detection {
    set: String,
    number: String,
}

#[derive(Debug, Clone)]
enum Message {
    Detecting(pokedetect::Event),
}

impl Detect {
    pub fn new() -> Self {
        Self::Loading
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Detecting(event) => match event {
                pokedetect::Event::Captured(image) => {
                    let detection = if let Self::Capturing { detection, .. } = self {
                        detection.clone()
                    } else {
                        None
                    };

                    *self = Self::Capturing {
                        image: image::Handle::from_rgba(image.width, image.height, image.rgba),
                        detection,
                    };
                }
                pokedetect::Event::Detected { set, number } => {
                    let Self::Capturing { detection, .. } = self else {
                        return;
                    };

                    *detection = Some(Detection { set, number });
                }
            },
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self {
            Self::Loading => text("Loading...").into(),
            Self::Capturing {
                image: handle,
                detection,
            } => {
                let capture = container(image(handle).width(Fill).height(Fill)).padding(10);

                if let Some(Detection { set, number }) = detection {
                    stack![
                        capture,
                        bottom_right(
                            container(text!("{set} {number}",).font(Font::MONOSPACE))
                                .padding(10)
                                .style(container::dark)
                        )
                        .padding(10),
                    ]
                    .into()
                } else {
                    capture.into()
                }
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(pokedetect::run).map(Message::Detecting)
    }
}
