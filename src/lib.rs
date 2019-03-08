use core::fmt::Write;
use std::f32;
use std::fmt;

type UInt = ::std::os::raw::c_uint;

pub trait Game where Self: Sized {
    /// The type of the actions.
    type Action: Eq;

    /// Returns the next state after action applied.
    fn next(&self, action: &Self::Action) -> Self;

    /// The the type of the all legal actions.
    type NextActions: IntoIterator<Item=Self::Action>;

    /// Returns the all legal actions.
    fn next_actions(&self) -> Self::NextActions;

    /// Returns the end status for the current player.
    fn status(&self) -> Status;

    /// Returns the priority to visit for MCTS.
    #[inline]
    fn priority(&self, win: UInt, visits: UInt, parent_visits: UInt) -> f32 {
        if visits == 0 {
            f32::INFINITY
        } else {
            win as f32 / visits as f32 + (2.0 * (parent_visits as f32).ln() / visits as f32).sqrt()
        }
    }
}

/// End status.
#[derive(Eq, PartialEq, Copy, Clone, Hash, Debug)]
pub enum Status {
    Unfinished,
    Win,
    Lose,
    Draw,
}

impl Default for Status {
    #[inline]
    fn default() -> Status {
        Status::Unfinished
    }
}

impl fmt::Display for Status {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Eq, PartialEq)]
enum Children<G: Game> {
    NotExpanded,
    Expanded(Vec<Node<G>>),
    PlayerWin(bool),
}

impl<G: Game> Children<G> {
    #[inline]
    fn has_expanded(&self) -> bool {
        match self {
            &Children::NotExpanded => false,
            _ => true,
        }
    }

    #[inline]
    fn expand(&mut self, game: &G, is_current_player: bool) where G: Game {
        if self.has_expanded() {
            return;
        }

        *self = match game.status() {
            Status::Win => Children::PlayerWin(is_current_player),
            Status::Draw => Children::PlayerWin(false),
            Status::Lose => Children::PlayerWin(!is_current_player),
            Status::Unfinished => Children::Expanded({
                let mut actions = game.next_actions();
                actions.into_iter().map(|a| Node::new(game.next(&a), !is_current_player, a)).collect()
            })
        }
    }
}

impl<G: Game> Clone for Children<G> where G: Clone, G::Action: Clone {
    #[inline]
    fn clone(&self) -> Self {
        match self {
            &Children::NotExpanded => Children::NotExpanded,
            &Children::Expanded(ref v) => Children::Expanded(v.clone()),
            &Children::PlayerWin(b) => Children::PlayerWin(b)
        }
    }
}

impl<G: Game> Default for Children<G> {
    #[inline]
    fn default() -> Children<G> {
        Children::NotExpanded
    }
}

impl<G: Game> fmt::Debug for Children<G> where G: fmt::Debug, G::Action: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Children::NotExpanded => f.write_str("NotExpanded"),
            &Children::Expanded(ref v) => {
                f.write_str("Expanded(")?;
                fmt::Debug::fmt(v, f)?;
                f.write_char(')')
            }
            &Children::PlayerWin(b) => {
                f.write_str("PlayerWin(")?;
                fmt::Debug::fmt(&b, f)?;
                f.write_char(')')
            }
        }
    }
}

#[derive(Eq, PartialEq, Clone, Default, Debug)]
struct Node<G: Game> {
    game: G,
    is_curr_player: bool,
    prev_act: G::Action,
    wins: UInt,
    visits: UInt,
    children: Children<G>,
}

impl<G: Game> Node<G> {
    #[inline]
    fn new(game: G, is_curr_player: bool, prev_act: G::Action) -> Node<G> {
        Node { game, is_curr_player, prev_act, wins: 0, visits: 0, children: Children::NotExpanded }
    }

    #[inline]
    fn leaf(game: G, is_curr_player: bool, prev_act: G::Action, has_player_win: bool) -> Node<G> {
        Node { game, is_curr_player, prev_act, wins: 0, visits: 0, children: Children::PlayerWin(has_player_win) }
    }

    #[inline]
    fn priority(&self, parent_visits: UInt) -> f32 {
        self.game.priority(self.wins, self.visits, parent_visits)
    }

    fn play_out(&mut self) -> bool {
        self.visits += 1;
        self.children.expand(&self.game, self.is_curr_player);

        let has_player_win: bool;
        match &mut self.children {
            &mut Children::NotExpanded => { panic!("unreachable") }
            &mut Children::PlayerWin(win) => has_player_win = win,
            &mut Children::Expanded(ref mut children) => {
                let (mut prior_child, children) = children.split_first_mut().unwrap();
                let mut max_priority = prior_child.priority(self.visits);
                if max_priority == f32::INFINITY {
                    has_player_win = prior_child.play_out();
                } else {
                    for child in children {
                        let priority = child.priority(self.visits);
                        if priority == f32::INFINITY {
                            // Need not write max_priority because it is not used after for loop.
                            prior_child = child;
                            break;
                        }

                        if priority > max_priority {
                            max_priority = priority;
                            prior_child = child;
                        }
                    }

                    has_player_win = prior_child.play_out();
                }
            }
        }

        if has_player_win {
            self.wins += 1
        }
        has_player_win
    }

    fn next(self, act: G::Action) -> Node<G> {
        match self.children {
            Children::NotExpanded => Node::new(self.game.next(&act), !self.is_curr_player, act),
            Children::PlayerWin(win) => panic!("game finished"),
            Children::Expanded(children) => {
                let mut node = None;
                for child in children {
                    if child.prev_act == act {
                        node = Some(child);
                    }
                }
                node.expect("action must contained in the return of Game::next_actions()")
            }
        }
    }
}

#[derive(Eq, PartialEq, Default)]
pub struct Mct<G: Game> {
    game: G,
    is_curr_player: bool,
    visits: UInt,
    children: Vec<Node<G>>,
}

impl<G: Game> Mct<G> {
    #[inline]
    pub fn new(game: G, is_current_player: bool) -> Mct<G> {
        Mct {
            children: game.next_actions().into_iter().map(|a| Node::new(game.next(&a), !is_current_player, a)).collect(),
            game,
            is_curr_player: is_current_player,
            visits: 0,
        }
    }

    /// Returns the number of times this node is visited.
    #[inline]
    pub fn visits(&self) -> UInt {
        self.visits
    }

    #[inline]
    pub fn is_current_player(&self) -> bool {
        self.is_curr_player
    }
}

impl<G: Game> Mct<G> {
    #[inline]
    pub fn play_out(&mut self) {
        self.visits += 1;

        if let Some((mut prior_child, children)) = self.children.split_first_mut() {
            let mut max_priority = prior_child.priority(self.visits);
            if max_priority == f32::INFINITY {
                prior_child.play_out();
                return;
            }

            for child in children {
                let priority = child.priority(self.visits);
                if priority == f32::INFINITY {
                    child.play_out();
                    return;
                }

                if priority > max_priority {
                    prior_child = child;
                    max_priority = priority;
                }
            }

            prior_child.play_out();
        }
    }

    pub fn next(&mut self, action: G::Action) {
        use std::mem;
        let mut node = None;
        let mut children = Vec::new();
        mem::swap(&mut children, &mut self.children);
        for child in children {
            if child.prev_act == action {
                node = Some(child);
            }
        }
        let mut node = node.expect("action must contained in the return of Game::next_actions()");

        self.is_curr_player = !self.is_curr_player;
        self.visits = node.visits;
        node.children.expand(&node.game, node.is_curr_player);
        self.children = match node.children {
            Children::NotExpanded => panic!("unreachable"),
            Children::Expanded(v) => v,
            Children::PlayerWin(b) => panic!("game finished")
        };
        self.game = node.game;
    }

    #[inline]
    pub fn best_action(&self) -> &G::Action {
        let (mut best_child, children) = self.children.split_first().expect("game finished");
        let mut max_win_rate = if best_child.visits == 0 {
            // not visited, not good
            0.0
        } else {
            best_child.wins as f32 / best_child.visits as f32
        };
        for child in children {
            if child.visits == 0 {
                continue;
            }

            let win_rate: f32 = child.wins as f32 / child.visits as f32;
            if win_rate > max_win_rate {
                best_child = child;
                max_win_rate = win_rate;
            }
        }

        &best_child.prev_act
    }
}
