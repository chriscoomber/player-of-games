extern crate tic_tac_toe;
extern crate game;

fn main() {
    let mut adjudicator = game::Adjudicator::new(
        tic_tac_toe::TicTacToe::new(),
        game::RandomPlayer(game::PlayerEnum::One),
        game::RandomPlayer(game::PlayerEnum::Two),
    );
    while adjudicator.conclusion().is_none() {
        adjudicator.progress_one_turn()
    }

    println!("Conclusion: {:?}", adjudicator.conclusion());
}