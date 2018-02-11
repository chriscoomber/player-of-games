extern crate rand;

pub trait Player<Game: GameState> {
    fn choose_move(&self, game: &Game) -> <Game as GameState>::Move;
}

pub struct RandomPlayer(pub PlayerEnum);

impl<Game: GameState> Player<Game> for RandomPlayer {
    fn choose_move(&self, game: &Game) -> <Game as GameState>::Move {
        random_sample(game.all_legal_moves(self.0)).expect("There were no legal moves")
    }
}

/// Returns None only if the iterator is empty.
///
/// Suppose there are N elements in the iterator.
/// Generate N bernoulli random variables, X~n~ with probability of success (1/n).
///
/// The chosen element is the n^th^ element, iff X~n~ && !X~n+1~ && ... && !X~N~. This has
/// probability of 1/n * (1-1/(n+1)) * ... * (1-1/N)). Some maths can show that this is 1/N for all
/// n, hence the sampling is fair.
///
/// We implement this using an algorithm which doesn't need to know N up front, and hence can be
/// used for any iterator.
///
///
/// (Borrowed from https://github.com/rust-lang/rust/issues/19639#issuecomment-66200471.)
fn random_sample<T, I: Iterator<Item = T>>(iter: I) -> Option<T> {
    let mut elem = None;
    let mut i = 1f64;
    for new_item in iter {
        if rand::random::<f64>() < (1f64/i) {
            elem = Some(new_item);
        }
        i += 1.0;
    }
    elem
}

#[derive(Clone, Copy, Debug)]
pub enum PlayerEnum {
    One,
    Two
}

impl PlayerEnum {
    fn other(&self) -> PlayerEnum {
        match *self {
            PlayerEnum::One => PlayerEnum::Two,
            PlayerEnum::Two => PlayerEnum::One,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Conclusion {
    Win(PlayerEnum),
    Draw
}

pub trait GameState: std::fmt::Debug + Clone + 'static {
    type Move: Copy;
    fn update(&mut self, game_move: Self::Move, player: PlayerEnum);
    fn update_with_closure<F: Fn(&Self) -> Self::Move>(&mut self, f: F, player: PlayerEnum) {
        let game_move = f(self);
        self.update(game_move, player);
    }
    fn all_legal_moves<'a>(&'a self, player: PlayerEnum) -> Box<Iterator<Item = Self::Move> + 'a>;
    fn try_conclude(&self, next_player: PlayerEnum) -> Option<Conclusion>;
}

pub struct Adjudicator<Game: GameState, PlayerOne: Player<Game>, PlayerTwo: Player<Game>> {
    current_turn: PlayerEnum,
    game_state: Game,
    player_one: PlayerOne,
    player_two: PlayerTwo,
    conclusion: Option<Conclusion>,
}

impl<Game: GameState, PlayerOne: Player<Game>, PlayerTwo: Player<Game>> Adjudicator<Game, PlayerOne, PlayerTwo> {
    pub fn new(game_state: Game, player_one: PlayerOne, player_two: PlayerTwo) -> Self {
        Self {
            current_turn: PlayerEnum::One,
            game_state,
            player_one,
            player_two,
            conclusion: None,
        }
    }

    pub fn progress_one_turn(&mut self) {
        match self.current_turn {
            PlayerEnum::One => {
                let player_one = &self.player_one;
                self.game_state.update_with_closure(|state| player_one.choose_move(state), PlayerEnum::One)
            },
            PlayerEnum::Two => {
                let player_two = &self.player_two;
                self.game_state.update_with_closure(|state| player_two.choose_move(state), PlayerEnum::Two)
            },
        }

        // Log out the new game state:
        println!("New game state: \n{:?}", self.game_state);

        let next_player = self.current_turn.other();

        match self.game_state.try_conclude(next_player) {
            Some(conclusion) => {
                self.conclusion = Some(conclusion);
                println!("Got conclusion: {:?}", conclusion)
            },
            None => self.current_turn = next_player,
        }
    }

    pub fn conclusion(&self) -> Option<Conclusion> {
        self.conclusion
    }
}
