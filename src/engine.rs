use std::{
    ops::{Index, IndexMut, Not},
    time::{Duration, Instant},
};

use rand::prelude::SliceRandom;
use shakmaty::{CastlingMode, Color, Move, Outcome, Position, Setup};

use crate::board::{Bughouse, BughousePositionError};

struct Node {
    side_that_moved: Color,
    last_move: Option<Move>,
    position: Bughouse,
    wins: f32,
    simulations: i32,
    children: Vec<NodeId>,
}

#[derive(Copy, Clone)]
struct NodeId(usize);

struct Tree {
    nodes: Vec<Node>,
}
impl Index<NodeId> for Tree {
    type Output = Node;
    fn index(&self, idx: NodeId) -> &Node {
        &self.nodes[idx.0]
    }
}

impl IndexMut<NodeId> for Tree {
    fn index_mut(&mut self, idx: NodeId) -> &mut Node {
        &mut self.nodes[idx.0]
    }
}

impl Tree {
    fn new(root: Node) -> Tree {
        Tree { nodes: vec![root] }
    }
    fn push_node(&mut self, node: Node) -> NodeId {
        let idx = self.nodes.len();
        self.nodes.push(node);
        NodeId(idx)
    }

    fn select_next(&self, node_id: NodeId) -> Option<NodeId> {
        let node = &self[node_id];
        let exploration_constant = 1.414; // sqrt(2) is theoretically ideal, but in practice this value is adjusted to maximize strength
        let uct = |child_id: NodeId| {
            let child = &self[child_id];
            if child.simulations == 0 {
                // Suggestions from around the internet say that the UCT score for unvisited nodes should be very high
                f32::MAX
            } else {
                child.wins / child.simulations as f32
                    + exploration_constant
                        * ((node.simulations as f32).ln() / child.simulations as f32).sqrt()
            }
        };
        node.children
            .iter()
            .fold(
                (None, -1f32),
                |(highest_uct_child, highest_uct): (Option<NodeId>, f32), &child_id| {
                    let uct_score = uct(child_id);
                    if uct_score > highest_uct {
                        (Some(child_id), uct_score)
                    } else {
                        (highest_uct_child, highest_uct)
                    }
                },
            )
            .0
    }

    // Selects an array of nodes from the root down to a leaf
    fn select_branch(&self, root: NodeId) -> Vec<NodeId> {
        let mut branch = vec![root];
        while let Some(next) = self.select_next(*branch.last().unwrap()) {
            branch.push(next);
        }
        branch
    }

    fn expand_tree(&mut self, leaf_id: NodeId) {
        let node = &mut self[leaf_id];
        let children: Vec<_> = node
            .position
            .legal_moves()
            .iter()
            .map(|legal_move| Node {
                last_move: Some(legal_move.clone()),
                side_that_moved: node.side_that_moved.not(),
                position: node
                    .position
                    .clone()
                    .play(&legal_move)
                    .expect("Illegal move played from legal move list"),
                wins: 0f32,
                simulations: 0,
                children: vec![],
            })
            .collect();
        let children_ids: Vec<_> = children
            .into_iter()
            .map(|node| self.push_node(node))
            .collect();

        self[leaf_id].children.extend(children_ids);
    }

    fn simulate(position: &Bughouse) -> Outcome {
        let mut simulation_board = position.clone();
        loop {
            if let Some(random_move) = simulation_board
                .legal_moves()
                .choose(&mut rand::thread_rng())
            {
                simulation_board = simulation_board
                    .play(random_move)
                    .expect("Illegal move played from legal move list");
            } else if let Some(outcome) = simulation_board.outcome() {
                break outcome;
            } else {
                panic!(
                    "No legal moves were found, but the game is not over (this should be impossible)"
                );
            }
        }
    }

    fn backpropagate(&mut self, branch: Vec<NodeId>, result: Outcome) {
        for node_id in branch {
            let node = &mut self[node_id];
            node.wins += match result {
                Outcome::Decisive { winner } => {
                    if winner == node.side_that_moved {
                        1f32
                    } else {
                        0f32
                    }
                }
                Outcome::Draw => 0.5f32,
            };
            node.simulations += 1;
        }
    }

    pub fn execute_mcts(&mut self) {
        let root_id = NodeId(0);
        let mut branch = self.select_branch(root_id);
        let leaf = *branch.last().expect("Branch should not be empty");
        self.expand_tree(leaf);
        if let Some(c) = self[leaf].children.choose(&mut rand::thread_rng()) {
            let outcome = Tree::simulate(&self[*c].position);
            branch.push(*c);
            self.backpropagate(branch, outcome);
        }
    }

    pub fn best_move(&self) -> Option<Move> {
        let root = &self[NodeId(0)];
        let best_child_id = root.children.iter().fold(
            None,
            |most_visited_child_or_none: Option<NodeId>, next_child_id| {
                if let Some(most_visited_child_id) = most_visited_child_or_none {
                    let most_visited_child = &self[most_visited_child_id];
                    let next_child = &self[*next_child_id];
                    println!("This node was simulated {} times", next_child.simulations);
                    if next_child.simulations > most_visited_child.simulations {
                        Some(*next_child_id)
                    } else {
                        Some(most_visited_child_id)
                    }
                } else {
                    Some(*next_child_id)
                }
            },
        )?;
        let best_move = self[best_child_id]
            .last_move
            .clone()
            .expect("Every node except the root should have a last_move");
        Some(best_move)
    }
}

pub enum Until {
    Milliseconds(u64),
    Iterations(usize),
}

pub struct Engine {
    tree: Tree,
}

impl Engine {
    pub fn new(setup: &dyn Setup) -> Result<Engine, BughousePositionError> {
        let position = Bughouse::from_setup(setup, CastlingMode::Standard)?;
        let root = Node {
            last_move: None,
            side_that_moved: position.turn().not(),
            position,
            wins: 0f32,
            simulations: 0,
            children: vec![],
        };
        Ok(Engine {
            tree: Tree::new(root),
        })
    }
    pub fn go(&mut self, until: Until) -> Option<Move> {
        let start = Instant::now();
        let mut iterations = 0;
        while match until {
            Until::Milliseconds(max_milliseconds) => {
                start.elapsed() < Duration::from_millis(max_milliseconds)
            }
            Until::Iterations(max_iterations) => iterations < max_iterations,
        } {
            self.tree.execute_mcts();
            iterations += 1;
        }
        println!("iterations: {}", iterations);
        self.tree.best_move()
    }
}
