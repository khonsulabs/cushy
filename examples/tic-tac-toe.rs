use std::fmt::Display;
use std::iter;
use std::ops::Not;
use std::time::SystemTime;

use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::button::ButtonKind;
use gooey::{Run, WithClone};
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let app = Dynamic::default();
    app.map_each(app.with_clone(|app| {
        move |state: &AppState| match state {
            AppState::Playing => play_screen(&app).make_widget(),
            AppState::Winner(winner) => game_end(*winner, &app).make_widget(),
        }
    }))
    .switcher()
    .contain()
    .width(Lp::inches(2)..Lp::inches(6))
    .height(Lp::inches(2)..Lp::inches(6))
    .centered()
    .run()
}

#[derive(Default, Debug, Eq, PartialEq)]
enum AppState {
    #[default]
    Playing,
    Winner(Option<Player>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Player {
    X,
    O,
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Player::X => f.write_str("X"),
            Player::O => f.write_str("O"),
        }
    }
}

impl Not for Player {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::X => Self::O,
            Self::O => Self::X,
        }
    }
}

struct GameState {
    app: Dynamic<AppState>,
    current_player: Player,
    cells: Vec<Option<Player>>,
}

impl GameState {
    fn new_game(app: &Dynamic<AppState>) -> Self {
        Self {
            app: app.clone(),
            // Bad RNG: if we have an even milliseconds in the current
            // timestamp, it's O's turn first.
            current_player: if SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("invalid system time")
                .as_millis()
                % 2
                == 0
            {
                Player::O
            } else {
                Player::X
            },
            cells: iter::repeat(None).take(9).collect(),
        }
    }

    fn play(&mut self, row: usize, column: usize) {
        let player = self.current_player;
        self.current_player = !player;

        self.cells[row * 3 + column] = Some(player);

        if let Some(winner) = self.check_for_winner() {
            self.app.set(AppState::Winner(Some(winner)));
        } else if self.cells.iter().all(Option::is_some) {
            self.app.set(AppState::Winner(None));
        }
    }

    fn check_for_winner(&self) -> Option<Player> {
        // Rows and columns
        for i in 0..3 {
            if let Some(winner) = self
                .winner_in_cells([[i, 0], [i, 1], [i, 2]])
                .or_else(|| self.winner_in_cells([[0, i], [1, i], [2, i]]))
            {
                return Some(winner);
            }
        }

        // Diagonals
        self.winner_in_cells([[0, 0], [1, 1], [2, 2]])
            .or_else(|| self.winner_in_cells([[2, 0], [1, 1], [0, 2]]))
    }

    fn winner_in_cells(&self, cells: [[usize; 2]; 3]) -> Option<Player> {
        match (
            self.cell(cells[0][0], cells[0][1]),
            self.cell(cells[1][0], cells[1][1]),
            self.cell(cells[2][0], cells[2][1]),
        ) {
            (Some(a), Some(b), Some(c)) if a == b && b == c => Some(a),
            _ => None,
        }
    }

    fn cell(&self, row: usize, column: usize) -> Option<Player> {
        self.cells[row * 3 + column]
    }
}

fn game_end(winner: Option<Player>, app: &Dynamic<AppState>) -> impl MakeWidget {
    let app = app.clone();
    let label = if let Some(winner) = winner {
        format!("{winner:?} wins!")
    } else {
        String::from("No winner")
    };

    label
        .h1()
        .and(
            "Play Again"
                .into_button()
                .on_click(move |_| {
                    app.set(AppState::Playing);
                })
                .into_default(),
        )
        .into_rows()
        .centered()
        .expand()
}

fn play_screen(app: &Dynamic<AppState>) -> impl MakeWidget {
    let game = Dynamic::new(GameState::new_game(app));
    let current_player_label = game.map_each(|state| format!("{}'s Turn", state.current_player));

    current_player_label.and(play_grid(&game)).into_rows()
}

fn play_grid(game: &Dynamic<GameState>) -> impl MakeWidget {
    row_of_squares(0, game)
        .expand()
        .and(row_of_squares(1, game).expand())
        .and(row_of_squares(2, game).expand())
        .into_rows()
}

fn row_of_squares(row: usize, game: &Dynamic<GameState>) -> impl MakeWidget {
    square(row, 0, game)
        .expand()
        .and(square(row, 1, game).expand())
        .and(square(row, 2, game).expand())
        .into_columns()
}

fn square(row: usize, column: usize, game: &Dynamic<GameState>) -> impl MakeWidget {
    let game = game.clone();
    let enabled = Dynamic::new(true);
    let label = Dynamic::default();
    (&enabled, &label).with_clone(|(enabled, label)| {
        game.for_each(move |state| {
            let Some(player) = state.cell(row, column) else {
                return;
            };

            if enabled.replace(false).is_some() {
                label.set(player.to_string());
            }
        });
    });

    label
        .clone()
        .into_button()
        .kind(ButtonKind::Outline)
        .on_click(move |_| game.lock().play(row, column))
        .with_enabled(enabled)
        .pad()
        .expand()
}
