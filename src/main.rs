use game::uci::UCIEngine;

mod engine;
mod game;

fn main() {
    let mut engine = UCIEngine::new();
    engine.run();
}
