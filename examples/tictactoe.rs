extern crate mcts;

use core::fmt::Write;
use std::fmt;
use std::mem;
use std::ops::Range;

use rand;
use rand::Rng;

use mcts::{Game, Status, Uct};

/// The type of bit board.
pub type Board = u16;

/// The type of positions where i, j is represented by i * j
pub type Pos = u8;

/// Lines in board.
const LINES: [Board; 8] = [
    0b111, 0b111_000, 0b111_000_000,  // horizontal
    0b001_001_001, 0b010_010_010, 0b100_100_100,  // vertical
    0b001_010_100, 0b100_010_001  // orthogonal
];

#[derive(Eq, PartialEq, Copy, Clone, Hash)]
pub struct TicTacToe {
    current: Board,
    next: Board,
    is_current_first: bool,
}

impl fmt::Debug for TicTacToe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TicTacToe")
            .field("current", &format_args!("{:09b}", self.current))
            .field("next", &format_args!("{:09b}", self.next))
            .field("is_current_first", &self.is_current_first)
            .finish()
    }
}

impl TicTacToe {
    #[inline]
    pub fn new() -> TicTacToe {
        TicTacToe { current: 0, next: 0, is_current_first: true }
    }

    #[inline]
    pub fn can_play_at(self, pos: Pos) -> bool {
        (self.current | self.next) & 1 << pos == 0
    }

    #[inline]
    pub fn played_at(self, pos: Pos) -> TicTacToe {
        TicTacToe { current: self.next, next: self.current | 1 << pos, is_current_first: !self.is_current_first }
    }

    #[inline]
    fn is_aligned(board: Board) -> bool {
        LINES.iter().any(|&line| line & board == line)
    }

    #[inline]
    fn swap(&mut self) {
        mem::swap(&mut self.current, &mut self.next)
    }

    #[inline]
    fn is_full(self) -> bool {
        self.current | self.next == 0b111_111_111
    }
}

impl Game for TicTacToe {
    type Action = Pos;

    #[inline]
    fn next(&self, action: &Pos) -> TicTacToe {
        TicTacToe { current: self.next, next: self.current | (1 << *action), is_current_first: !self.is_current_first }
    }

    type NextActions = Vec<Pos>;

    fn next_actions(&self) -> Vec<Pos> {
        let mut v = Vec::with_capacity(9);
        for i in 0..9 {
            if self.can_play_at(i) {
                v.push(i);
            }
        }
        v
    }

    #[inline]
    fn status(&self) -> Status {
        if TicTacToe::is_aligned(self.current) {
            Status::Win
        } else if TicTacToe::is_aligned(self.next) {
            Status::Lose
        } else if self.is_full() {
            Status::Draw
        } else {
            Status::Unfinished
        }
    }
}

impl fmt::Display for TicTacToe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(" -- -- --")?;
        for i in 0..3 {
            f.write_str("\n|")?;
            for j in 0..3 {
                let mask = 1 << (i * 3 + j);
                let (c, n) = if self.is_current_first { ("[]", "><") } else { ("><", "[]") };
                if self.current & mask == mask {
                    f.write_str(c)?;
                } else if self.next & mask == mask {
                    f.write_str(n)?;
                } else {
                    f.write_str("  ")?;
                }
                f.write_char('|')?;
            }
            f.write_str("\n -- -- --")?;
        }
        Ok(())
    }
}

fn main() {
    let mut game = TicTacToe::new();
    let mut mct = Uct::new(game, true);

    loop {
        let action = if game.is_current_first {
            for i in 0..100 {
                mct.play_out();
            }
            *mct.most_visited()
        } else {
            use std::i16;
            let p = random_best_play(game);
            p
        };
        game = game.played_at(action);
        mct.next(action);
        println!("{}", game);

        match game.status() {
            Status::Unfinished => {}
            _ => break
        }
    }
}

/// Returns value of `game` using alpha-beta pruning.
///
/// value is 2 ^ {depth to end}.
fn alpha_beta(game: TicTacToe, alpha: i16, beta: i16) -> i16 {
    fn alpha_beta_with_depth(game: TicTacToe, alpha: i16, beta: i16, depth: i16) -> i16 {
        if TicTacToe::is_aligned(game.next) { return -(1 << depth); }
        if game.is_full() { return 0; }
        let mut alpha = alpha;
        for pos in 0..9 {
            if !game.can_play_at(pos) { continue; }
            let next_alpha = -alpha_beta_with_depth(game.played_at(pos), -beta, -alpha, depth + 1);
            alpha = next_alpha.max(alpha);
            if alpha >= beta { break; }
        }
        alpha
    }

    alpha_beta_with_depth(game, alpha, beta, 0)
}


// Because of evaluation function, even when can win on next turn, will not play at winning position.
/// Returns best action randomly.
pub fn random_best_play(game: TicTacToe) -> Pos {
    // minimize opponent's value
    use std::i16;

    let mut min_pos = Vec::new();
    let mut min_val = i16::MAX - 1;
    for pos in 0..9 {
        if !game.can_play_at(pos) { continue; }

        let val = alpha_beta(game.played_at(pos), -(i16::MAX - 1), min_val + 1);
        if val == min_val {
            min_pos.push(pos);
        } else if val < min_val {
            min_pos.clear();
            min_pos.push(pos);
            min_val = val;
        }
    }
    *rand::thread_rng().choose(&min_pos[..]).unwrap_or_else(||
        panic!("Specified game has no playable position")
    )
}
