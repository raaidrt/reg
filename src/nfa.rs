pub mod node;
use char_stream::CharStream;
use node::Node;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct NFA {
    states: usize,
    starting: HashSet<Node>,
    delta: HashMap<(Node, char), HashSet<Node>>,
    finished: HashSet<Node>,
}

impl NFA {
    pub fn is_match(&self, stream: &mut CharStream) -> bool {
        let mut nodes: HashSet<Node> = self.starting.clone();
        for ch in stream {
            let mut new_nodes: HashSet<Node> = HashSet::new();
            for &node in nodes.iter() {
                if let Some(set) = self.delta.get(&(node, ch)) {
                    for &new_node in set.iter() {
                        new_nodes.insert(new_node);
                    }
                }
            }
            nodes = new_nodes;
        }
        nodes.iter().any(|node| self.finished.contains(node))
    }
}

pub fn plus(first: &NFA, second: &NFA) -> NFA {
    let increase = |&node| {
        let Node(n) = node;
        Node(n + first.states)
    };
    let states = first.states + second.states;
    let starting = first
        .starting
        .union(&second.starting.iter().map(increase).collect())
        .copied()
        .collect();
    let finished = first
        .finished
        .union(&second.finished.iter().map(increase).collect())
        .copied()
        .collect();

    let mut delta = first.delta.clone();

    for (&(Node(n), ch), set) in second.delta.iter() {
        let set = set.iter().map(increase).collect();
        delta.insert((Node(n + first.states), ch), set);
    }

    NFA {
        states,
        starting,
        delta,
        finished,
    }
}

pub fn times(first: &NFA, second: &NFA) -> NFA {
    let states = first.states + second.states - first.finished.len();
    let starting = first.starting.clone();
    let increase = |&node: &Node| -> Node {
        let Node(n) = node;
        Node(n + first.states - first.finished.len())
    };
    // any nodes mapping to a first.finished state should map to second.starting states as well
    let mut delta = first.delta.clone();
    let finished: HashSet<Node> = second.finished.iter().map(increase).collect();
    let second_starting: HashSet<Node> = second.starting.clone().iter().map(increase).collect();
    for (&(Node(n), ch), set) in first.delta.iter() {
        let mut new_set: HashSet<Node> = HashSet::new();
        let mut added_second_starting = false;
        for &Node(m) in set.iter() {
            if first.finished.contains(&Node(m)) {
                if !added_second_starting {
                    added_second_starting = true;
                    let mut tmp = new_set.clone();
                    for &Node(p) in second_starting.iter() {
                        tmp.insert(Node(p));
                    }
                    new_set = tmp;
                }
            } else {
                new_set.insert(Node(m));
            }
        }
        delta.insert((Node(n), ch), new_set);
    }

    for (&(Node(n), ch), set) in second.delta.iter() {
        let new_set: HashSet<Node> = set.iter().map(increase).collect();
        delta.insert((increase(&Node(n)), ch), new_set);
    }

    NFA {
        states,
        starting,
        delta,
        finished,
    }
}

pub fn unit(ch: char) -> NFA {
    NFA {
        states: 2,
        starting: [Node(0)].into(),
        delta: [((Node(0), ch), [Node(1)].into())].into(),
        finished: [Node(1)].into(),
    }
}

pub fn star(nfa: &NFA) -> NFA {
    let mut finished = nfa.finished.clone();
    nfa.starting.iter().for_each(|&Node(n)| {
        finished.insert(Node(n));
    });
    let mut delta = nfa.delta.clone();
    for &(Node(n), ch) in nfa.delta.keys() {
        let mapped = nfa.delta.get(&(Node(n), ch));
        match mapped {
            Some(set) => {
                let mut new_set = set.clone();
                let added_starting = false;
                for &Node(m) in set.iter() {
                    if nfa.finished.contains(&Node(m)) && !added_starting {
                        for &Node(p) in nfa.starting.iter() {
                            new_set.insert(Node(p));
                        }
                    }
                }
                delta.insert((Node(n), ch), new_set);
            }
            _ => {}
        }
    }
    NFA {
        states: nfa.states,
        starting: nfa.starting.clone(),
        delta,
        finished,
    }
}

pub fn empty() -> NFA {
    NFA {
        states: 1,
        starting: [Node(0)].into(),
        delta: [].into(),
        finished: [Node(0)].into(),
    }
}

#[cfg(test)]
mod test {
    use crate::nfa::*;
    #[test]
    pub fn test_empty() {
        let nfa = empty();
        let mut stream = CharStream::from_string(String::from(""));
        assert!(nfa.is_match(&mut stream));
    }
    #[test]
    pub fn test_nonempty_rejects() {
        let nfa = empty();
        let mut stream = CharStream::from_string(String::from("a"));
        assert!(!nfa.is_match(&mut stream));
    }
    #[test]
    pub fn test_single_char() {
        let nfa = unit('a');
        let mut stream = CharStream::from_string(String::from("a"));
        assert!(nfa.is_match(&mut stream));
    }
    #[test]
    pub fn test_nonsinglechar_rejects() {
        let nfa = unit('a');
        let mut stream = CharStream::from_string(String::from("aa"));
        assert!(!nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from(""));
        assert!(!nfa.is_match(&mut stream));
    }
    #[test]
    pub fn test_times() {
        let nfa = times(&unit('a'), &unit('b'));
        let mut stream = CharStream::from_string(String::from("ab"));
        assert!(nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("ba"));
        assert!(!nfa.is_match(&mut stream));
        let another_nfa = times(&empty(), &nfa);
        stream = CharStream::from_string(String::from("ab"));
        assert!(another_nfa.is_match(&mut stream));
    }
    #[test]
    pub fn test_plus() {
        let nfa = plus(
            &times(&unit('a'), &unit('b')),
            &times(&unit('c'), &unit('d')),
        );
        let mut stream1 = CharStream::from_string(String::from("ab"));
        let mut stream2 = CharStream::from_string(String::from("cd"));
        let mut stream3 = CharStream::from_string(String::from("ac"));
        let mut stream4 = CharStream::from_string(String::from("cb"));
        print!("The nfa is {:?}", nfa);
        assert!(nfa.is_match(&mut stream1));
        assert!(nfa.is_match(&mut stream2));
        assert!(!nfa.is_match(&mut stream3));
        assert!(!nfa.is_match(&mut stream4));
    }
    #[test]
    pub fn test_star_simple() {
        let nfa = star(&unit('a'));
        let mut stream = CharStream::from_string(String::from(""));
        assert!(nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("a"));
        assert!(nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("aa"));
        assert!(nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("aba"));
        assert!(!nfa.is_match(&mut stream));
    }
}
