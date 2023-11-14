use gooey::kludgine::app::winit::keyboard::Key;
use gooey::kludgine::figures::units::Px;
use gooey::kludgine::figures::{Point, Rect, Size};
use gooey::kludgine::render::Renderer;
use gooey::kludgine::shapes::Shape;
use gooey::kludgine::tilemap::{Object, ObjectLayer, TileKind, TileMapFocus, Tiles, TILE_SIZE};
use gooey::kludgine::Color;
use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::{TileMap, Label, Stack};
use gooey::{Run, Tick};

const PLAYER_SIZE: Px = Px(16);

#[rustfmt::skip]
const TILES: [TileKind; 64] = {
    const O: TileKind = TileKind::Color(Color::PURPLE);
    const X: TileKind = TileKind::Color(Color::WHITE);
    [
        X, X, X, X, X, X, X, X,
        X, O, O, O, O, O, O, X,
        X, O, X, O, O, X, O, X,
        X, O, O, O, O, O, O, X,
        X, O, X, O, O, X, O, X,
        X, O, O, X, X, O, O, X,
        X, O, O, O, O, O, O, X,
        X, X, X, X, X, X, X, X,
    ]
};

fn main() -> gooey::Result {
    let mut characters = ObjectLayer::new();

    let myself = characters.push(Player {
        color: Color::RED,
        position: Point::new(TILE_SIZE.0 as f32, TILE_SIZE.0 as f32),
    });

    let layers = Dynamic::new((Tiles::new(8, 8, TILES), characters));

    let debug_message = Dynamic::new(String::new());

    let tilemap = TileMap::dynamic(layers.clone())
        .focus_on(TileMapFocus::Object {
            layer: 1,
            id: myself,
        })
        .tick(Tick::times_per_second(60, move |elapsed, input| {
            // get mouse cursor position and subsequently get the object under
            // the mouse

            let mut direction = Point::new(0., 0.);
            if input.keys.contains(&Key::ArrowDown) {
                direction.y += 1.0;
            }
            if input.keys.contains(&Key::ArrowUp) {
                direction.y -= 1.0;
            }
            if input.keys.contains(&Key::ArrowRight) {
                direction.x += 1.0;
            }
            if input.keys.contains(&Key::ArrowLeft) {
                direction.x -= 1.0;
            }

            let one_second_movement = direction * TILE_SIZE.0 as f32;

            layers.map_mut(|layers| {
                layers.1[myself].position += Point::new(
                    one_second_movement.x * elapsed.as_secs_f32(),
                    one_second_movement.y * elapsed.as_secs_f32(),
                )
            });
        }));

    Stack::rows(tilemap.debug_output(debug_message.clone()).expand().and(Label::new(debug_message))).run()
}

#[derive(Debug)]
struct Player {
    color: Color,
    position: Point<f32>,
}

impl Object for Player {
    fn position(&self) -> Point<Px> {
        self.position.cast()
    }

    fn render(&self, center: Point<Px>, zoom: f32, context: &mut Renderer<'_, '_>) {
        let zoomed_size = PLAYER_SIZE * zoom;
        context.draw_shape(
            &Shape::filled_rect(
                Rect::new(
                    Point::new(-zoomed_size / 2, -zoomed_size / 2),
                    Size::squared(zoomed_size),
                ),
                self.color,
            ),
            center,
            None,
            None,
        )
    }
}
