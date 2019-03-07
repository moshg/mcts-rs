use std::f32;
use std::fmt;

type UInt = ::std::os::raw::c_uint;

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

pub trait Game where Self: Sized {
    fn status(&self) -> Status;

    fn children(&self) -> Vec<Self>;

    #[inline]
    fn priority(&self, win: UInt, visits: UInt, parent_visits: UInt) -> f32 {
        if visits == 0 {
            f32::INFINITY
        } else {
            win as f32 / visits as f32 + (2.0 * (parent_visits as f32).ln() / visits as f32).sqrt()
        }
    }
}

#[derive(Eq, PartialEq, Clone, Hash, Debug)]
enum Children<G> {
    NotExpanded,
    Expanded(Vec<Node<G>>),
    Leaf(bool),
}

impl<G> Default for Children<G> {
    #[inline]
    fn default() -> Children<G> {
        Children::NotExpanded
    }
}

impl<G> Children<G> {
    #[inline]
    fn has_expanded(&self) -> bool {
        match self {
            Children::NotExpanded => false,
            _ => true,
        }
    }

    #[inline]
    fn expand(&mut self, game: &G) where G: Game {
        if self.has_expanded() {
            return;
        }

        *self = match game.status() {
            Status::Win => Children::Leaf(true),
            Status::Draw => Children::Leaf(false),
            Status::Lose => Children::Leaf(false),
            Status::Unfinished => Children::Expanded(
                game.children().into_iter().map(|g| Node::new(g)).collect()
            )
        }
    }
}

#[derive(Eq, PartialEq, Clone, Default, Hash, Debug)]
struct Node<G> {
    game: G,
    wins: UInt,
    visits: UInt,
    children: Children<G>,
}

impl<G> Node<G> {
    #[inline]
    fn new(game: G) -> Node<G> {
        Node { game, wins: 0, visits: 0, children: Children::NotExpanded }
    }
}

impl<G: Game> Node<G> {
    #[inline]
    fn priority(&self, parent_visits: UInt) -> f32 {
        self.game.priority(self.wins, self.visits, parent_visits)
    }

    pub fn play_out(&mut self) -> bool {
        self.visits += 1;
        self.children.expand(&self.game);

        let has_win: bool;
        match &mut self.children {
            &mut Children::NotExpanded => { panic!("unreachable") }
            &mut Children::Leaf(win) => has_win = win,
            &mut Children::Expanded(ref mut children) => {
                let (mut prior_child, children) = children.split_first_mut().unwrap();
                let mut max_priority = prior_child.priority(self.visits);
                if max_priority == f32::INFINITY {
                    has_win = prior_child.play_out();
                } else {
                    for child in children {
                        let priority = child.priority(self.visits);
                        if priority == f32::INFINITY {
                            let has_win = child.play_out();
                            if has_win {
                                self.wins += 1
                            }
                            return has_win;
                        }

                        if priority > max_priority {
                            max_priority = priority;
                            prior_child = child;
                        }
                    }

                    has_win = prior_child.play_out();
                }
            }
        }

        if has_win {
            self.wins += 1
        }
        has_win
    }
}

#[derive(Eq, PartialEq, Clone, Default, Hash, Debug)]
pub struct Mct<G>(Node<G>);

impl<G> Mct<G> {
    #[inline]
    pub fn new(game: G) -> Mct<G> {
        Mct(Node::new(game))
    }

    #[inline]
    pub fn wins(&self) -> UInt {
        self.0.wins
    }

    #[inline]
    pub fn play_outs(&self) -> UInt {
        self.0.visits
    }
}

impl<G: Game> Mct<G> {
    #[inline]
    pub fn play_out(&mut self) {
        self.0.play_out();
    }
}
