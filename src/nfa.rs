pub mod node;
use node::Node;
use char_stream::CharStream;
use std::collections::HashSet;

pub struct NFA<T> where T : Fn(&Node, char) -> HashSet<Node> {
    states: usize,
    starting: HashSet<Node>,
    delta: T,
    finished: HashSet<Node>
}

impl<T> NFA<T> where T : Fn(&Node, char) -> HashSet<Node> {
    pub fn is_match(&self, s : String) -> bool {
        let mut nodes : HashSet<Node> = self.starting.clone();
        let mut stream = CharStream::from_string(s);
        while let Some(ch) = stream.next() {
            let mut new_nodes : HashSet<Node> = HashSet::new();
            for node in nodes.iter() {
                for new_node in (self.delta)(node, ch) {
                    new_nodes.insert(new_node);
                }
            }
            nodes = new_nodes;
        }
        let mut matched = false;
        for node in nodes.iter() {
            if self.finished.contains(node) {
                matched = true;
                break;
            }
        }
        return matched;
    }
    pub fn plus(&self, other : &NFA<impl Fn(&Node, char) -> HashSet<Node>>) -> NFA<impl Fn(&Node, char) -> HashSet<Node>> {
        let increase = |&node| {
            let Node(n) = node;
            return Node(n + self.states);
        };
        let states = self.states + other.states;
        let starting = self.starting.union(&other.starting.iter().map(increase).collect()).copied().collect();
        let delta = Box::new(|&node : &Node, ch : char| -> HashSet<Node> {
            let Node(n) = node;
            if n < self.states {
                return (self.delta)(&node, ch);
            } else {
                return (other.delta)(&Node(n - self.states), ch);
            }
        });
        let finished = self.finished.union(&other.finished.iter().map(increase).collect()).copied().collect();
        NFA {
            states,
            starting, 
            delta, 
            finished
        }
    }
}

