use gooey::kludgine::app::winit::keyboard::Key;
use gooey::kludgine::figures::units::Px;
use gooey::kludgine::figures::{Point, Rect, Size};
use gooey::kludgine::render::Renderer;
use gooey::kludgine::shapes::Shape;
use gooey::kludgine::tilemap::{Object, ObjectLayer, TileKind, TileMapFocus, Tiles, TILE_SIZE};
use gooey::kludgine::Color;
use gooey::value::Dynamic;
use gooey::widgets::TileMap;
use gooey::{Run, Tick};
use kludgine::app::winit::keyboard::NamedKey;
use kludgine::figures::FloatConversion;
use kludgine::DrawableExt;

const PLAYER_SIZE: Px = Px::new(16);

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
        position: Point::new(TILE_SIZE.into_float(), TILE_SIZE.into_float()),
    });

    let layers = Dynamic::new((Tiles::new(8, 8, TILES), characters));

    TileMap::dynamic(layers.clone())
        .focus_on(TileMapFocus::Object {
            layer: 1,
            id: myself,
        })
        .tick(Tick::times_per_second(60, move |elapsed, input| {
            let mut direction = Point::new(0., 0.);
            if input.keys.contains(&Key::Named(NamedKey::ArrowDown)) {
                direction.y += 1.0;
            }
            if input.keys.contains(&Key::Named(NamedKey::ArrowUp)) {
                direction.y -= 1.0;
            }
            if input.keys.contains(&Key::Named(NamedKey::ArrowRight)) {
                direction.x += 1.0;
            }
            if input.keys.contains(&Key::Named(NamedKey::ArrowLeft)) {
                direction.x -= 1.0;
            }

            let one_second_movement = direction * TILE_SIZE.into_float();

            layers.map_mut(|layers| {
                layers.1[myself].position += Point::new(
                    // TODO fix this in figures
                    one_second_movement.x * elapsed.as_secs_f32(),
                    one_second_movement.y * elapsed.as_secs_f32(),
                )
            });
        }))
        .run()
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
            Shape::filled_rect(
                Rect::new(Point::squared(-zoomed_size / 2), Size::squared(zoomed_size)),
                self.color,
            )
            .translate_by(center),
        )
    }
}
