extern crate game;
#[macro_use]
extern crate ndarray;

use std::fmt;
use std::ops::Deref;

use ndarray::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Piece {
    Nought,
    Cross,
}

impl From<game::PlayerEnum> for Piece {
    fn from(player: game::PlayerEnum) -> Self {
        match player {
            game::PlayerEnum::One => Piece::Cross,
            game::PlayerEnum::Two => Piece::Nought,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct OptionalPiece(Option<Piece>);

impl From<Option<Piece>> for OptionalPiece {
    fn from(t: Option<Piece>) -> Self {
        OptionalPiece(t)
    }
}

impl Deref for OptionalPiece {
    type Target = Option<Piece>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for OptionalPiece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self.0 {
            Some(Piece::Nought) => "O",
            Some(Piece::Cross) => "X",
            None => "_",

        })
    }
}

#[derive(Clone)]
pub struct TicTacToe {
    state: Array2<OptionalPiece>
}

impl TicTacToe {
    pub fn new() -> Self {
        Self {
            state: Array::from_elem((3, 3), None.into())
        }
    }

    fn count(&self, piece: OptionalPiece) -> u8 {
        self.state.iter().fold(0u8, |n, x| {
            if *x == piece {
                n + 1
            } else {
                n
            }
        })
    }

    fn does_piece_win(&self, piece: Piece) -> bool {
        // Any columns all match?
        for column in self.state.axis_iter(ndarray::Axis(0)) {
            if column.iter().all(|x| *x == Some(piece).into()) {
                return true;
            }
        }
        // Any rows all match?
        for row in self.state.axis_iter(ndarray::Axis(1)) {
            if row.iter().all(|x| *x == Some(piece).into()) {
                return true;
            }
        }
        // Diagonal matches?
        if self.state.diag().iter().all(|x| *x == Some(piece).into()) {
            return true;
        }
        // Anti-diagonal matches? Invert one of the axis and take a look at the diag again.
        let mut view = self.state.view();
        view.invert_axis(ndarray::Axis(0));
        if view.diag().iter().all(|x| *x == Some(piece).into()) {
            return true;
        }

        // Otherwise
        false
    }

    fn is_legal(&self, game_move: Move, player: game::PlayerEnum) -> Result<(), String> {
        let Move {
            coordinates: (x, y),
            piece,
        } = game_move;

        match (player, piece) {
            (game::PlayerEnum::One, Piece::Nought) => return Err("Player 1 tried to place noughts".to_string()),
            (game::PlayerEnum::Two, Piece::Cross) => return Err("Player 2 tried to place crosses".to_string()),
            _ => ()
        }

        if self.state[[x, y]].is_some() {
            return Err("Trying to override another piece".to_string());
        }

        let count_noughts = self.count(Some(Piece::Nought).into());
        let count_crosses = self.count(Some(Piece::Cross).into());
        match piece {
            Piece::Nought => {
                // Check that there's one more Cross
                if !(count_noughts == count_crosses - 1) {
                    return Err("Nought playing out of turn".to_string())
                }
            }

            Piece::Cross => {
                // Check that there's the same number of either
                if count_noughts != count_crosses {
                    return Err("Crosses playing out of turn".to_string())
                }
            }
        }

        Ok(())
    }
}

impl fmt::Debug for TicTacToe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TicTacToe {{\n{}\n}}", self.state)
    }
}

/// Coordinates are guaranteed to be 0,1,2
#[derive(PartialEq, Clone, Copy)]
pub struct Move {
    coordinates: (usize, usize),
    piece: Piece,
}

impl Move {
    pub fn new(x: usize, y:usize, piece: Piece) -> Move {
        if x > 2 || y > 2 {
            panic!("Coordinates were out of bounds.")
        }
        Move {
            coordinates: (x, y),
            piece
        }
    }
}

impl game::GameState for TicTacToe {
    type Move = Move;

    fn update(&mut self, game_move: Self::Move, player: game::PlayerEnum) {
        self.is_legal(game_move, player).expect("Move not legal");

        let Move {
            coordinates: (x, y),
            piece,
        } = game_move;

        self.state[[x, y]] = Some(piece).into();
    }

    fn all_legal_moves<'a>(&'a self, player: game::PlayerEnum) -> Box<Iterator<Item = Move> + 'a> {
        let game_clone = self.clone();
        let closure = move |((x, y), _)| {
            let game_move = Move::new(x, y, Piece::from(player));
            if game_clone.is_legal(game_move, player).is_ok() {
                return Some(game_move);
            } else {
                return None
            }
        };
        Box::new(self.state.indexed_iter().filter_map(closure))
    }

    fn try_conclude(&self, next_player: game::PlayerEnum) -> Option<game::Conclusion> {
        if self.does_piece_win(Piece::Cross.into()) {
            return Some(game::Conclusion::Win(game::PlayerEnum::One))
        }
        if self.does_piece_win(Piece::Nought.into()) {
            return Some(game::Conclusion::Win(game::PlayerEnum::Two))
        }

        // Otherwise, if there are no moves left for the next player, draw
        if self.all_legal_moves(next_player).count() == 0 {
            return Some(game::Conclusion::Draw)
        }

        // Otherwise, the game goes on
        None
    }
}

