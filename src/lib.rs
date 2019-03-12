use core::fmt::Write;
use std::f32;
use std::fmt;

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
    fn bias_const(&self) -> f32 {
        2.0f32.sqrt()
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

#[derive(PartialEq, Clone, Default)]
struct Node<G: Game> {
    game: G,
    prev_act: G::Action,
    visits: f32,
    wins: f32,
    children: Children<G>,
}

impl<G: Game> Node<G> {
    #[inline]
    fn new(game: G, prev_act: G::Action) -> Node<G> {
        Node { game, prev_act, wins: 0.0, visits: 0.0, children: Children::NotExpanded }
    }

    #[inline]
    fn leaf(game: G, prev_act: G::Action, win: f32) -> Node<G> {
        Node { game, prev_act, wins: 0.0, visits: 0.0, children: Children::Leaf(win) }
    }

    #[inline]
    fn priority(&self, parent_visits: f32) -> f32 {
        if self.visits == 0.0 {
            f32::INFINITY
        } else {
            self.wins / self.visits + self.game.bias_const() * (parent_visits.ln() / self.visits).sqrt()
        }
    }

    fn play_out(&mut self) -> f32 {
        self.visits += 1.0;
        self.children.expand(&self.game);

        let win: f32;
        match &mut self.children {
            &mut Children::NotExpanded => { panic!("unreachable") }
            &mut Children::Leaf(w) => win = w,
            &mut Children::Expanded(ref mut children) => {
                let (mut prior_child, children) = children.split_first_mut().unwrap();
                let mut max_priority = prior_child.priority(self.visits);
                if max_priority == f32::INFINITY {
                    win = 1.0 - prior_child.play_out();
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

                    win = 1.0 - prior_child.play_out();
                }
            }
        }

        self.wins += win;
        win
    }

    fn next(self, act: G::Action) -> Node<G> {
        match self.children {
            Children::NotExpanded => Node::new(self.game.next(&act), act),
            Children::Leaf(win) => panic!("game finished"),
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

impl<G: Game> fmt::Debug for Node<G> where G: fmt::Debug, G::Action: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Node")
            .field("prev_action", &self.prev_act)
            .field("visits", &self.visits)
            .field("wins", &self.wins)
            .field("game", &self.game)
            .field("children", &self.children)
            .finish()
    }
}

#[derive(PartialEq)]
enum Children<G: Game> {
    NotExpanded,
    Expanded(Vec<Node<G>>),
    Leaf(f32),
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
    fn expand(&mut self, game: &G) where G: Game {
        if self.has_expanded() {
            return;
        }

        *self = match game.status() {
            // Current player of `game` has been changed when `game.next()` called.
            // So player who do previous action is different from current player.
            Status::Win => Children::Leaf(0.0),
            Status::Draw => Children::Leaf(0.5),
            Status::Lose => Children::Leaf(1.0),
            Status::Unfinished => Children::Expanded({
                let mut actions = game.next_actions();
                actions.into_iter().map(|a| Node::new(game.next(&a), a)).collect()
            })
        }
    }
}

impl<G: Game> Clone for Children<G> where G: Clone, G::Action: Clone {
    #[inline]
    fn clone(&self) -> Children<G> {
        match self {
            &Children::NotExpanded => Children::NotExpanded,
            &Children::Expanded(ref v) => Children::Expanded(v.clone()),
            &Children::Leaf(b) => Children::Leaf(b)
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
                f.debug_tuple("Expanded")
                    .field(v)
                    .finish()
            }
            &Children::Leaf(ref b) => {
                f.debug_tuple("Leaf")
                    .field(b)
                    .finish()
            }
        }
    }
}

/// Upper confidence bound 1 applied to Tree Search.
#[derive(PartialEq, Default)]
pub struct Uct<G: Game> {
    game: G,
    visits: f32,
    children: Vec<Node<G>>,
}

impl<G: Game> Uct<G> {
    #[inline]
    pub fn new(game: G, is_current_player: bool) -> Uct<G> {
        Uct {
            children: game.next_actions().into_iter().map(|a| Node::new(game.next(&a), a)).collect(),
            game,
            visits: 0.0,
        }
    }

    /// Returns the number of times this node is visited.
    #[inline]
    pub fn visits(&self) -> u32 {
        self.visits as u32
    }
}

impl<G: Game> Uct<G> {
    #[inline]
    pub fn play_out(&mut self) {
        self.visits += 1.0;

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

        if self.children.is_empty() {
            panic!("game finished");
        }

        let mut node = None;
        let mut children = Vec::new();
        mem::swap(&mut children, &mut self.children);
        for child in children {
            if child.prev_act == action {
                node = Some(child);
            }
        }
        let mut node = node.expect("action must contained in the return of Game::next_actions()");

        self.visits = node.visits;
        node.children.expand(&node.game);
        self.children = match node.children {
            Children::NotExpanded => panic!("unreachable"),
            Children::Expanded(v) => v,
            Children::Leaf(b) => Vec::new()
        };
        self.game = node.game;
    }

    #[inline]
    pub fn most_visited(&self) -> &G::Action {
        let (mut best_child, children) = self.children.split_first().expect("game finished");
        let mut max_visits = best_child.visits;
        for child in children {
            if child.visits > max_visits {
                best_child = child;
                max_visits = child.visits;
            }
        }

        &best_child.prev_act
    }
}

impl<G: Game> fmt::Debug for Uct<G> where G: fmt::Debug, G::Action: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Uct")
            .field("game", &self.game)
            .field("visits", &self.visits)
            .field("children", &self.children)
            .finish()
    }
}
