use std::array;
use std::cmp::Ordering;
use std::time::Duration;

use gooey::kludgine::app::winit::keyboard::Key;
use gooey::kludgine::figures::units::Px;
use gooey::kludgine::figures::{Point, Rect, Size};
use gooey::kludgine::render::Renderer;
use gooey::kludgine::shapes::Shape;
use gooey::kludgine::tilemap::{
    DebugGrid, Object, ObjectLayer, TileArray, TileKind, TileMapFocus, TILE_SIZE,
};
use gooey::kludgine::Color;
use gooey::value::Dynamic;
use gooey::widgets::TileMap;
use gooey::{Run, Tick};
use kludgine::app::winit::keyboard::NamedKey;
use kludgine::figures::FloatConversion;
use kludgine::sprite::{Sprite, SpriteSource};
use kludgine::{include_aseprite_sprite, DrawableExt};

const PLAYER_SIZE: Px = Px::new(16);

fn main() -> gooey::Result {
    let mut characters = ObjectLayer::new();

    let mut sprite = include_aseprite_sprite!("assets/stickguy").unwrap();
    sprite.set_current_tag(Some("Idle")).unwrap();

    let myself = characters.push(Player {
        sprite,
        current_frame: None,
        hovered: false,
        position: Point::new(TILE_SIZE.into_float(), TILE_SIZE.into_float()),
    });

    let sprite = include_aseprite_sprite!("assets/grass").unwrap();

    let layers = Dynamic::new((
        TileArray::new(
            8,
            array::from_fn::<_, 64, _>(|_| TileKind::Sprite(sprite.clone())),
        ),
        characters,
        DebugGrid,
    ));

    let tilemap = TileMap::dynamic(layers.clone())
        .focus_on(TileMapFocus::Object {
            layer: 1,
            id: myself,
        })
        .tick(Tick::times_per_second(60, move |elapsed, input| {
            // get mouse cursor position and subsequently get the object under the cursor

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

            let cursor_pos = input.mouse.as_ref().map(|mouse| mouse.position);

            layers.map_mut(|layers| {
                let player = &mut layers.1[myself];

                let animation_tag = match direction.x.total_cmp(&0.) {
                    Ordering::Less => "WalkLeft",
                    Ordering::Equal => "Idle",
                    Ordering::Greater => "WalkRight",
                };
                player
                    .sprite
                    .set_current_tag(Some(animation_tag))
                    .expect("valid tag");

                player.current_frame =
                    Some(player.sprite.get_frame(Some(elapsed)).expect("valid tag"));

                player.position += one_second_movement * elapsed.as_secs_f32();

                let rect = Rect::new(player.position - Size::squared(8.), Size::squared(16.));
                layers.1[myself].hovered =
                    cursor_pos.map_or(false, |cursor_pos| rect.cast().contains(cursor_pos));
            });
        }));

    tilemap.run()
}

#[derive(Debug)]
struct Player {
    sprite: Sprite,
    current_frame: Option<SpriteSource>,
    hovered: bool,
    position: Point<f32>,
}

impl Object for Player {
    fn position(&self) -> Point<Px> {
        self.position.cast()
    }

    fn render(
        &self,
        center: Point<Px>,
        zoom: f32,
        context: &mut Renderer<'_, '_>,
    ) -> Option<Duration> {
        let zoomed_size = PLAYER_SIZE * zoom;
        if self.hovered {
            context.draw_shape(
                Shape::filled_rect(
                    Rect::new(Point::squared(-zoomed_size / 2), Size::squared(zoomed_size)),
                    Color::new(255, 255, 255, 80),
                )
                .translate_by(center),
            );
        }

        if let Some(frame) = &self.current_frame {
            context.draw_texture(
                frame,
                Rect::new(center - zoomed_size / 2, Size::squared(zoomed_size)),
                1.,
            );
        }

        self.sprite.remaining_frame_duration().ok().flatten()
    }
}
