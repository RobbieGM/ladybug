use std::str::FromStr;

use engine::Engine;
use shakmaty::fen::Fen;

mod board;
mod engine;

fn main() {
    let fen = Fen::from_str("6k1/5ppp/3r4/8/8/8/2R2PPP/6K1 w - - 0 1").expect("invalid fen");
    let mut engine = Engine::new(&fen).expect("Engine failed to be created");
    println!("Finding best first move for white");
    println!("{:?}", engine.go(4_000));
}
