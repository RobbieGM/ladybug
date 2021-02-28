use std::str::FromStr;

use engine::{Engine, Until};
use shakmaty::{
    fen::{self, Fen},
    Chess, FromSetup,
};

mod board;
mod engine;

fn main() {
    let fen = Fen::from_str("7k/8/b5K1/3R4/8/1P4P1/8/8 w - - 0 1").expect("invalid fen");
    let mut engine = Engine::new(&fen).expect("Engine failed to be created");
    println!("Finding best first move for white");
    println!("{:?}", engine.go(Until::Milliseconds(10_000)));
}
