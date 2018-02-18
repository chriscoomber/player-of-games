extern crate daggy;
extern crate game;

use std::rc::{Rc, Weak};
use std::collections::HashMap;
use std::sync::RwLock;

struct Node<Game: game::GameState> {
    pub player: game::PlayerEnum,
    pub local_attempts: u8,
    pub local_wins: u8,
    pub local_losses: u8,
    /// Known children (some may be unknown)
    pub children: HashMap<<Game as game::GameState>::Move, Game>,
    /// Known parents - many may be unknown.
    pub parents: HashMap<<Game as game::GameState>::Move, Game>,
    debug_attempts: RwLock<u8>,
    debug_wins: RwLock<u8>,
    debug_losses: RwLock<u8>
}

impl<Game: game::GameState> std::fmt::Debug for Node<Game> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Node {{ player: {:?}, attempts: {}, wins: {}, losses: {}, children: {} }}", self.player, self.debug_attempts.read().unwrap(), self.debug_wins.read().unwrap(), self.debug_losses.read().unwrap(), self.children.len())
    }
}

impl<Game: game::GameState> Node<Game> {
    fn new(player: game::PlayerEnum, parent: Option<(<Game as game::GameState>::Move, Game)>) -> Self {
        Self {
            player,
            local_attempts: 0,
            local_wins: 0,
            local_losses: 0,
            children: HashMap::new(),
            parents: {
                let mut map = HashMap::new();
                match parent {
                    Some((k ,v)) => {
                        map.insert(k, v);
                    },
                    _ => ()
                }
                map
            },
            debug_attempts: RwLock::new(0),
            debug_wins: RwLock::new(0),
            debug_losses: RwLock::new(0),
        }
    }

    fn tree_attempts(&self, cache: &HashMap<Game, Node<Game>>) -> HashMap<Game, u8> {
        let map = self.children.values().fold(HashMap::new(), |mut map, child| {
            let child_node = cache.get(child).expect("Dangling pointer");
            map.extend(child_node.tree_attempts(cache));
            map.insert(child.clone(), child_node.local_attempts);
            map
        });
        *self.debug_attempts.write().unwrap() = map.values().sum();
        map
    }

    fn attempts(&self, cache: &HashMap<Game, Node<Game>>) -> u8 {
        self.tree_attempts(cache).values().sum()
    }

    fn tree_wins(&self, cache: &HashMap<Game, Node<Game>>) -> HashMap<Game, u8> {
        let map = self.children.values().fold(HashMap::new(), |mut map, child| {
            let child_node = cache.get(child).expect("Dangling pointer");
            map.extend(child_node.tree_losses(cache));
            map.insert(child.clone(), child_node.local_losses);
            map
        });
        *self.debug_wins.write().unwrap() = map.values().sum();
        map
    }

    fn wins(&self, cache: &HashMap<Game, Node<Game>>) -> u8 {
        self.tree_wins(cache).values().sum()
    }

    fn tree_losses(&self, cache: &HashMap<Game, Node<Game>>) -> HashMap<Game, u8> {
        let map = self.children.values().fold(HashMap::new(), |mut map, child| {
            let child_node = cache.get(child).expect("Dangling pointer");
            map.extend(child_node.tree_wins(cache));
            map.insert(child.clone(), child_node.local_wins);
            map
        });
        *self.debug_losses.write().unwrap() = map.values().sum();
        map
    }

    fn losses(&self, cache: &HashMap<Game, Node<Game>>) -> u8 {
        self.tree_losses(cache).values().sum()
    }

    fn uct_value(&self, parent_attempts: u8, c: f64, cache: &HashMap<Game, Node<Game>>) -> f64 {
        let attempts = self.attempts(cache);

        // If never explored, maximum exploration value
        if attempts == 0 {
            return std::f64::MAX;
        }

        let exploitation_value = (self.wins(cache) as f64)/(attempts as f64);
        let exploration_value = c * ( (parent_attempts as f64).ln() / (attempts as f64) ).sqrt();

//        println!("UCT value was {} = {} + {} for {:?}", exploitation_value + exploration_value, exploitation_value, exploration_value, self);

        exploitation_value + exploration_value
    }

    fn choose_move_by_uct_value(&self, c: f64, game: &Game, cache: &HashMap<Game, Node<Game>>) -> Option<<Game as game::GameState>::Move> {
        #[derive(PartialOrd, PartialEq)]
        struct OrdF64(f64);

        impl OrdF64 {
            fn new(x: f64) -> Self {
                if x.is_nan() {
                    panic!("x is NAN");
                }
                OrdF64(x)
            }
        }

        impl Eq for OrdF64 {}

        impl Ord for OrdF64 {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.partial_cmp(&other.0).expect("f64 could not be compared")
            }
        }

        let attempts = self.attempts(cache);
        game.all_legal_moves(self.player).map(|game_move| {
            // Try to find a child with this move
            match self.children.get(&game_move) {
                Some(child) => {
                    // Get the UCT value for that child.
                    // FIXME: this can choose an unknown child which is actually explored quite a lot...
                    let uct_value = cache.get(child).expect("Dangling pointer").uct_value(attempts, c, cache);
                    (game_move, uct_value)
                }
                None => (game_move, std::f64::MAX)
            }
        }).max_by_key(|&(a, x)| OrdF64::new(x)).map(|x| x.0)
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

#[derive(Debug)]
pub struct MonteCarloTreeSearchPlayer<Game: game::GameState> {
    player: game::PlayerEnum,
    c: f64,
    explored_states: HashMap<Game, Node<Game>>,
    last_turn: Option<Game>,
}

impl<Game: game::GameState> MonteCarloTreeSearchPlayer<Game> {
    pub fn new(player: game::PlayerEnum, c: f64) -> Self {
        Self {
            player,
            c,
            explored_states: HashMap::new(),
            last_turn: None,
        }
    }

    /// Check that the following laws are obeyed
    ///
    /// - known parent / known child is mutual
    /// - if node has a known child or known parent, it has not been destroyed by pruning
    fn audit(&self) {
//        println!("Audit\n\n");
//        for (state, node) in self.explored_states.iter() {
//            println!("  Checking state: {:?} {:?}\n\n", state, node);
//            for parent in node.parents.values() {
//                let parent_node = self.explored_states.get(parent).expect("Parent didn't exist");
//                println!("      Checking parent: {:?} {:?}\n\n", parent, parent_node);
//                assert!(parent_node.children.values().any(|x| x == state));
//            }
//            for child in node.children.values() {
//                let child_node = self.explored_states.get(child).expect("Child didn't exist");
//                println!("      Checking child: {:?} {:?}\n\n", child, child_node);
//                assert!(child_node.parents.values().any(|x| x == state));
//            }
//        }
    }

    fn remove_tree(&mut self, game_state: Game) {
        // Remove this node
        let node = match self.explored_states.remove(&game_state) {
            Some(x) => x,
            None => return
        };

        // Remove node as child from all parents (... should be none)
        for (m, parent) in node.parents.iter() {
            self.explored_states.get_mut(parent).expect("Dangling pointer").children.remove(m);
        }

        // Remove node as parent from all children
        for (m, child) in node.children.iter() {
            self.explored_states.get_mut(child).expect("Dangling pointer").parents.remove(m);
        }

        // Iterate into orphans
        for (_, child) in node.children {
            if self.explored_states.get(&child).expect("Dangling pointer").parents.is_empty() {
                // Orphan
                self.remove_tree(child);
            }
        }
    }

    /// Remove game states which are now impossible.
    ///
    /// The best we can do is remove any top-level games that were not realized.
    ///
    /// This is allowed to be pretty slow, as we only do this once.
    fn pruning(&mut self, current_state: Option<Game>, game_move: &<Game as game::GameState>::Move) {
        let current_state = match current_state {
            Some(x) => x,
            None => return
        };

        // Remove the current game state, since it's been invalidated by this move.
        let current_node = match self.explored_states.remove(&current_state) {
            Some(x) => x,
            None => return
        };

        // Remove self as child from all parents (... should be none)
        for (m, parent) in current_node.parents.iter() {
            self.explored_states.get_mut(parent).expect("Dangling pointer").children.remove(m);
        }

        // Remove self as parent from all children
        for (m, child) in current_node.children.iter() {
            self.explored_states.get_mut(child).expect("Dangling pointer").parents.remove(m);
        }

        // Remove any unrealized children who are now orphans. Hopefully, if our pruning is good,
        // this will be all unrealized children.
        for child in current_node.children.into_iter().filter_map(|(m, g)| if m != *game_move { Some(g) } else { None }) {
            if self.explored_states.get(&child).expect("Dangling pointer").parents.is_empty() {
                // Orphan
                self.remove_tree(child);
            } else {
                println!("Warning: unrealized child that was not an orphan: {:?} {:?}", child, self.explored_states.get(&child));

            }
        }
    }

    /// Select the next node to look at.
    ///
    /// Starting with the current game state, do the following:
    ///
    /// 1) Make a node for the current game state if required.
    /// 2) Choose one of its legal moves using the uct value
    /// 3) If the move corresponds to a child, then repeat from step 2 for that child. Otherwise,
    ///    create a node for that child and select it.
    fn selection_and_expansion(&mut self, game: Game) -> Game {
        let mut current_parent: Option<(<Game as game::GameState>::Move, Game)> = None;
        let mut current_state = game;
        let mut current_player = self.player;

        loop {
            // Create the current state, if it doesn't already exist.
            if self.explored_states.get(&current_state).is_none() {
                self.explored_states.insert(current_state.clone(), Node::new(current_player, current_parent.clone()));
            } else {
                match current_parent.clone() {
                    Some((game_move, parent)) => {
                        self.explored_states.get_mut(&current_state).unwrap().parents.insert(game_move, parent);
                    },
                    None => ()
                }
            }

            // Make sure that the parent points to this move
            match current_parent {
                Some((game_move, state)) => {
                    self.explored_states.get_mut(&state).expect("Blah").children.insert(game_move, current_state.clone());
                }
                _ => ()
            }

            // If this is a leaf with 0 attempts, or there are no legal moves, use this. Else choose a legal move.
            let chosen_move = {
                let mut current_node = self.explored_states.get(&current_state).unwrap();

                if current_node.is_leaf() && current_node.local_attempts == 0 {
                    return current_state;
                }

                let chosen_move = current_node.choose_move_by_uct_value(self.c, &current_state, &self.explored_states);

                match chosen_move {
                    Some(chosen_move) => chosen_move,
                    None => return current_state,
                }
            };

            // Got a new move, iterate down
            current_parent = Some((chosen_move, current_state.clone()));
            current_state.update(chosen_move, current_player);
            current_player = current_player.other();
        }
    }
}

impl<Game: game::GameState> game::Player<Game> for MonteCarloTreeSearchPlayer<Game> {
    fn choose_move(&mut self, game: Game) -> <Game as game::GameState>::Move {
        // FIXME: time based rather than fixed number of searches.
        for _ in 1..100 {
            // selection and expansion
            let state_to_explore = self.selection_and_expansion(game.clone());
            self.audit();

            let node_to_explore = self.explored_states.get_mut(&state_to_explore).expect("Dangling pointer!");

            // Simulation and backpropogation
            let mut state = state_to_explore;
            let mut player = game::RandomPlayer(node_to_explore.player);
            loop {
                let current_player = player.0;

                match (state.try_conclude(current_player), node_to_explore.player) {
                    (Some(game::Conclusion::Win(game::PlayerEnum::One)), game::PlayerEnum::One) | (Some(game::Conclusion::Win(game::PlayerEnum::Two)), game::PlayerEnum::Two) => {
                        node_to_explore.local_wins += 1;
                        node_to_explore.local_attempts += 1;
                        break;
                    }
                    (Some(game::Conclusion::Win(_)), _) => {
                        node_to_explore.local_losses += 1;
                        node_to_explore.local_attempts += 1;
                        break;
                    }
                    (Some(game::Conclusion::Draw), _) => {
                        // FIXME: count draws as neither win nor loss???
                        node_to_explore.local_attempts += 1;
                        break;
                    }
                    (None, _) => ()
                }

                state.update_with_closure(|state| player.choose_move(state.clone()), current_player);
                player = game::RandomPlayer(current_player.other());
            }
        }

        // Pick the child with the most simulations made.
        let current_node = self.explored_states.get(&game).expect("Bleh");
        let decision = current_node.children.iter().map(|(m, child)| {
            (m, self.explored_states.get(child).unwrap().attempts(&self.explored_states))
        }).max_by_key(|&(m, x)| x).unwrap().0.clone();

        println!("Made decision: {:?}.\n\n{:?}", decision, self);
        decision
    }

    fn inform_of_move_played(&mut self, new_state: Game, game_move: &<Game as game::GameState>::Move) {
        let last_turn = self.last_turn.take();
        self.last_turn = Some(new_state);
        self.pruning(last_turn, game_move);
    }
}
