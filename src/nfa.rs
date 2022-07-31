pub mod node;
use char_stream::CharStream;
use node::Node;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct NFA {
    states: usize,
    starting: HashSet<Node>,
    delta: HashMap<(Node, ExtendedChar), HashSet<Node>>,
    finished: HashSet<Node>,
}

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub enum ExtendedChar {
    Char(char),
    Wildcard
}

impl NFA {
    pub fn is_match(&self, stream: &mut CharStream) -> bool {
        let mut nodes: HashSet<Node> = self.starting.clone();
        for ch in stream {
            let mut new_nodes: HashSet<Node> = HashSet::new();
            for &node in nodes.iter() {
                if let Some(set) = self.delta.get(&(node, ExtendedChar::Char(ch))) {
                    for &new_node in set.iter() {
                        new_nodes.insert(new_node);
                    }
                }
                if let Some(set) = self.delta.get(&(node, ExtendedChar::Wildcard)) {
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
    let states = first.states + second.states;
    let increase = |&node: &Node| -> Node {
        let Node(n) = node;
        return Node(n + first.states);
    };
    let mut starting = first.starting.clone();
    if first.starting.iter().any(|&node| {
        first.finished.contains(&node)
    }) {
        starting = starting.union(&second.starting.iter().map(increase).collect()).copied().collect();
    }
    // any nodes mapping to a first.finished state should map to second.starting states as well
    let mut delta = first.delta.clone();
    let finished: HashSet<Node> = second.finished.clone().iter().map(increase).collect();
    let second_starting: HashSet<Node> = second.starting.clone().iter().map(increase).collect();
    for (&(Node(n), ch), set) in first.delta.iter() {
        let mut new_set: HashSet<Node> = set.clone();
        if set.iter().any(|&node| {
            first.finished.contains(&node)
        }) {
            new_set = new_set.union(&second_starting).copied().collect();
        }
        delta.insert((Node(n), ch), new_set);
    }

    second.delta.iter().for_each(|(&(node, ch), set)| {
        let new_set = set.iter().map(increase).collect();
        delta.insert((increase(&node), ch), new_set);
    });

    NFA {
        states,
        starting,
        delta,
        finished,
    }
}

pub fn unit(ec: ExtendedChar) -> NFA {
    NFA {
        states: 2,
        starting: [Node(0)].into(),
        delta: [((Node(0), ec), [Node(1)].into())].into(),
        finished: [Node(1)].into(),
    }
}

pub fn star(nfa: &NFA) -> NFA {
    let mut finished = nfa.finished.clone();
    let mut delta = nfa.delta.clone();
    for (&(Node(n), ch), set) in nfa.delta.iter() {
        let mut new_set = set.clone();
        if set.iter().any(|&node| {
            nfa.finished.contains(&node)
        }) {
            new_set = new_set.union(&nfa.starting).copied().collect();
        }
        delta.insert((Node(n), ch), new_set);
    }
    nfa.starting.iter().for_each(|&Node(n)| {
        finished.insert(Node(n));
    });

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

    pub fn test_within_bounds(nfa: &NFA) {
        for &Node(n) in nfa.starting.iter() {
            assert!(n < nfa.states);
        }
        for &Node(n) in nfa.finished.iter() {
            assert!(n < nfa.states);
        }
    }

    #[test]
    pub fn test_empty() {
        let nfa = empty();
        test_within_bounds(&nfa);
        let mut stream = CharStream::from_string(String::from(""));
        assert!(nfa.is_match(&mut stream));
    }

    #[test]
    pub fn test_nonempty_rejects() {
        let nfa = empty();
        test_within_bounds(&nfa);
        let mut stream = CharStream::from_string(String::from("a"));
        assert!(!nfa.is_match(&mut stream));
    }

    #[test]
    pub fn test_single_char() {
        let nfa = unit(ExtendedChar::Char('a'));
        test_within_bounds(&nfa);
        let mut stream = CharStream::from_string(String::from("a"));
        assert!(nfa.is_match(&mut stream));
    }

    #[test]
    pub fn test_nonsinglechar_rejects() {
        let nfa = unit(ExtendedChar::Char('a'));
        test_within_bounds(&nfa);
        let mut stream = CharStream::from_string(String::from("aa"));
        assert!(!nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from(""));
        assert!(!nfa.is_match(&mut stream));
    }

    #[test]
    pub fn test_times() {
        let nfa = times(&unit(ExtendedChar::Char('a')), &unit(ExtendedChar::Char('b')));
        test_within_bounds(&nfa);
        println!("NFA ab is {:?}", nfa);
        let mut stream = CharStream::from_string(String::from("ab"));
        assert!(nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("ba"));
        assert!(!nfa.is_match(&mut stream));
        let another_nfa = times(&empty(), &nfa);
        stream = CharStream::from_string(String::from("ab"));
        println!("Another nfa is {:?}", another_nfa);
        assert!(another_nfa.is_match(&mut stream));
    }

    #[test]
    pub fn test_plus() {
        let nfa = plus(
            &times(&unit(ExtendedChar::Char('a')), &unit(ExtendedChar::Char('b'))),
            &times(&unit(ExtendedChar::Char('c')), &unit(ExtendedChar::Char('d'))),
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
    pub fn test_wildcard() {
        let nfa = times(
            &unit(ExtendedChar::Wildcard),
            &unit(ExtendedChar::Wildcard)
        );
        let mut stream = CharStream::from_string(String::from("a"));
        assert!(!nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("ab"));
        assert!(nfa.is_match(&mut stream));
    }

    #[test]
    pub fn test_star_simple() {
        let nfa = star(&unit(ExtendedChar::Char('a')));
        test_within_bounds(&nfa);
        println!("NFA a* is {:?}", nfa);
        let mut stream = CharStream::from_string(String::from(""));
        assert!(nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("a"));
        assert!(nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("aa"));
        assert!(nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("aba"));
        assert!(!nfa.is_match(&mut stream));
        let another_nfa = times(&star(&times(&unit(ExtendedChar::Char('a')), &unit(ExtendedChar::Char('b')))), &star(&unit(ExtendedChar::Char('c'))));
        println!("NFA2 is {:?}", another_nfa);
        stream = CharStream::from_string(String::from("ababab"));
        assert!(another_nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("ababccc"));
        assert!(another_nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from("abb"));
        assert!(!another_nfa.is_match(&mut stream));
    }

    #[test]
    pub fn test_star_with_plus_and_times() {
        let nfa = times(&star(&plus(&unit(ExtendedChar::Char('a')), &unit(ExtendedChar::Char('b')))), &star(&unit(ExtendedChar::Char('c'))));
        let mut stream1 = CharStream::from_string(String::from("a"));
        let mut stream2 = CharStream::from_string(String::from("abababbbaba"));
        let mut stream3 = CharStream::from_string(String::from("abababbbabaccc"));
        let mut stream4 = CharStream::from_string(String::from("ababaaaababbaccbc"));
        assert!(nfa.is_match(&mut stream1));
        assert!(nfa.is_match(&mut stream2));
        assert!(nfa.is_match(&mut stream3));
        assert!(!nfa.is_match(&mut stream4));
        test_within_bounds(&nfa);
        let another_nfa = star(&times(&unit(ExtendedChar::Char('a')), &unit(ExtendedChar::Char('b'))));
        test_within_bounds(&another_nfa);
        let mut stream = CharStream::from_string(String::from("abb"));
        assert!(!another_nfa.is_match(&mut stream));
    }
}
