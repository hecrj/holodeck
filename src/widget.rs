use iced::widget::{canvas, text};
use iced::{Element, Pixels};

pub fn logo<'a, Message>(size: impl Into<Pixels>) -> Element<'a, Message> {
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");

    let mut name = String::with_capacity(PKG_NAME.len());
    name.push(
        PKG_NAME
            .chars()
            .next()
            .expect("Non-empty name")
            .to_ascii_uppercase(),
    );
    name.push_str(&PKG_NAME[1..]);

    text(name).size(size).into()
}

pub fn pokeball<'a, Message: 'a>(size: impl Into<Pixels>) -> Element<'a, Message> {
    use iced::mouse;
    use iced::{Point, Rectangle, Renderer, Size, Theme};

    struct Pokeball;

    impl<Message> canvas::Program<Message> for Pokeball {
        type State = canvas::Cache;

        fn draw(
            &self,
            cache: &Self::State,
            renderer: &Renderer,
            theme: &Theme,
            bounds: Rectangle,
            _cursor: mouse::Cursor,
        ) -> Vec<canvas::Geometry> {
            let pokeball = cache.draw(renderer, bounds.size(), |frame| {
                const RADIUS: f32 = 100.0;
                const LINE: f32 = 30.0;

                let palette = theme.palette();

                let center = Point::new(RADIUS, RADIUS);
                let outer_circle = canvas::Path::circle(center, RADIUS);
                let inner_circle = canvas::Path::circle(center, RADIUS / 2.0);
                let button = canvas::Path::circle(center, RADIUS / 4.0);

                let line = Rectangle::new(
                    Point::new(0.0, RADIUS - LINE / 2.0),
                    Size::new(2.0 * RADIUS, LINE),
                );

                let scale = (bounds.width - 0.5) / (2.0 * RADIUS);

                frame.scale(scale);

                frame.fill(&outer_circle, palette.text);
                frame.fill(&inner_circle, palette.background);
                frame.fill_rectangle(line.position(), line.size(), palette.background);
                frame.fill(&button, palette.text);
            });

            vec![pokeball]
        }
    }

    let size = size.into();

    canvas(Pokeball).width(size).height(size).into()
}
