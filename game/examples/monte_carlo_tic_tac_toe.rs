extern crate tic_tac_toe;
extern crate game;
extern crate player_of_games;

fn main() {
    let mut adjudicator = game::Adjudicator::new(
        tic_tac_toe::TicTacToe::new(),
        player_of_games::MonteCarloTreeSearchPlayer::new(game::PlayerEnum::One, 2f64.sqrt()),
        player_of_games::MonteCarloTreeSearchPlayer::new(game::PlayerEnum::Two, 2f64.sqrt()),
    );
    while adjudicator.conclusion().is_none() {
        adjudicator.progress_one_turn()
    }

    println!("Conclusion: {:?}", adjudicator.conclusion());
}