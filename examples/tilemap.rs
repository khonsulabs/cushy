use gooey::dynamic::Dynamic;
use gooey::kludgine::app::winit::keyboard::Key;
use gooey::kludgine::figures::units::Px;
use gooey::kludgine::figures::{Point, Rect, Size};
use gooey::kludgine::render::Renderer;
use gooey::kludgine::shapes::Shape;
use gooey::kludgine::tilemap::{
    Object, ObjectId, ObjectLayer, TileKind, TileMapFocus, Tiles, TILE_SIZE,
};
use gooey::kludgine::Color;
use gooey::widget::{EventHandling, HANDLED, UNHANDLED};
use gooey::widgets::TileMap;
use gooey::{EventLoopError, Run};

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

fn main() -> Result<(), EventLoopError> {
    let mut characters = ObjectLayer::new();

    let myself = characters.push(Player {
        color: Color::RED,
        position: Point::new(TILE_SIZE * 1, TILE_SIZE * 1),
    });

    let layers = Dynamic::new((Tiles::new(8, 8, TILES), characters));

    TileMap::dynamic(layers.clone())
        .focus_on(TileMapFocus::Object {
            layer: 1,
            id: myself,
        })
        .on_key(move |key| handle_key(key, myself, &layers))
        .run()
}

fn handle_key(
    key: Key,
    player: ObjectId,
    layers: &Dynamic<(Tiles, ObjectLayer<Player>)>,
) -> EventHandling {
    let offset = match key {
        Key::ArrowDown => Point::new(Px(0), Px(1)),
        Key::ArrowUp => Point::new(Px(0), Px(-1)),
        Key::ArrowLeft => Point::new(Px(-1), Px(0)),
        Key::ArrowRight => Point::new(Px(1), Px(0)),
        _ => return UNHANDLED,
    };

    layers.map_mut(|layers| layers.1[player].position += offset);

    HANDLED
}

#[derive(Debug)]
struct Player {
    color: Color,
    position: Point<Px>,
}

impl Object for Player {
    fn position(&self) -> Point<Px> {
        self.position
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
