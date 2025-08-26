use game::uci::UCIEngine;

mod engine;
mod game;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut engine = UCIEngine::new();

    if args.len() > 1 {
        match args[1].as_str() {
            "bench" => {
                engine.bench();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[1]);
                std::process::exit(1);
            }
        }
    }

    engine.run();
}
