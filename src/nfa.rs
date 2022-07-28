pub mod node;
use node::Node;
use char_stream::CharStream;
use std::collections::{ HashSet, HashMap };

#[derive(Debug)]
pub struct NFA {
    states: usize,
    starting: HashSet<Node>,
    delta: HashMap<(Node, char), HashSet<Node>>,
    finished: HashSet<Node>
}

impl NFA {
    pub fn is_match(&self, stream : &mut CharStream) -> bool {
        let mut nodes: HashSet<Node> = self.starting.clone();
        while let Some(ch) = stream.next() {
            let mut new_nodes : HashSet<Node> = HashSet::new();
            for &node in nodes.iter() {
                match self.delta.get(&(node, ch)) {
                    Some(set) => {
                        for &new_node in set.iter() {
                            new_nodes.insert(new_node);
                        }
                    },
                    _ => { }
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
}

pub fn plus(first: &NFA, second: &NFA) -> NFA {
    let increase = |&node| {
        let Node(n) = node;
        return Node(n + first.states);
    };
    let states = first.states + second.states;
    let starting = first.starting.union(&second.starting.iter().map(increase).collect()).copied().collect();
    let finished = first.finished.union(&second.finished.iter().map(increase).collect()).copied().collect();

    let mut delta = first.delta.clone();
    
    for &(Node(n), ch) in second.delta.keys() {
        match second.delta.get(&(Node(n), ch)) {
            Some(other_set) => {
                let set = other_set.iter().map(increase).collect();
                delta.insert((Node(n + first.states), ch), set);
            },
            None => {
                panic!("This line should not execute");
            }
        }
    }

    NFA {
        states,
        starting, 
        delta, 
        finished
    }
}

pub fn times(first: &NFA, second: &NFA) -> NFA {
    let states = first.states + second.states;
    let mut starting = first.starting.clone();
    for &Node(n) in first.starting.iter() {
        if first.finished.contains(&Node(n)) {
            second.starting.iter().for_each(|&Node(m)| {
                starting.insert(Node(m + first.states));
            });
        }
    }
    let increase = |&node : &Node| -> Node {
        let Node(n) = node;
        return Node(n + first.states);
    };
    // any nodes mapping to a first.finished state should map to second.starting states as well
    let mut delta = first.delta.clone();
    let finished: HashSet<Node> = second.finished.clone().iter().map(increase).collect();
    let second_starting: HashSet<Node> = second.starting.clone().iter().map(increase).collect();
    for &(Node(n), ch) in first.delta.keys() {
        let mapped = first.delta.get(&(Node(n), ch));
        match mapped {
            Some(set) => {
                let mut new_set: HashSet<Node> = set.clone();
                let mut added_second_starting = false;
                for &Node(m) in set.iter() {
                    if first.finished.contains(&Node(m)) {
                        if !added_second_starting {
                            added_second_starting = true;
                            for &Node(p) in second_starting.iter() {
                                new_set.insert(Node(p));
                            }
                        }
                    } 
                }
                delta.insert((Node(n), ch), new_set);
            },
            _ => { }
        }
    }
    for &(Node(n), ch) in second.delta.keys() {
        let mapped = second.delta.get(&(Node(n), ch));
        match mapped {
            Some(set) => {
                let new_set: HashSet<Node> = set.iter().map(increase).collect();
                delta.insert((increase(&Node(n)), ch), new_set);
            },
            _ => { }
        }
    }
    NFA {
        states, 
        starting, 
        delta, 
        finished
    }
}

pub fn unit(ch : char) -> NFA {
    let mut starting : HashSet<Node> = HashSet::new();
    starting.insert(Node(0));
    let states = 2;
    let mut finished : HashSet<Node> = HashSet::new();
    finished.insert(Node(1));
    let mut delta : HashMap<(Node, char), HashSet<Node>> = HashMap::new();
    delta.insert((Node(0), ch), finished.clone());
    NFA {
        states,
        starting,
        delta,
        finished
    }
}

pub fn star(nfa: &NFA) -> NFA {
    let mut finished = nfa.finished.clone();
    let mut delta = nfa.delta.clone();
    for &(Node(n), ch) in nfa.delta.keys() {
        let mapped = nfa.delta.get(&(Node(n), ch));
        if let Some(set) = mapped {
            let mut new_set = set.clone();
            let mut added_starting = false;
            for &Node(m) in set.iter() {
                if nfa.finished.contains(&Node(m)) && !added_starting {
                    added_starting = true;
                    nfa.starting.iter().for_each(|&Node(p)| {
                        new_set.insert(Node(p));
                    });
                }
            }
            delta.insert((Node(n), ch), new_set);
        }
    }
    nfa.starting.iter().for_each(|&Node(n)| {
        finished.insert(Node(n));
    });

    NFA {
        states: nfa.states,
        starting: nfa.starting.clone(), 
        delta,
        finished
    }
}

pub fn empty() -> NFA {
    let mut starting = HashSet::new();
    starting.insert(Node(0));
    let delta = HashMap::new();
    let mut finished = HashSet::new();
    finished.insert(Node(0));
    NFA {
        states: 1,
        starting,
        delta, 
        finished
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
        let nfa = unit('a');
        test_within_bounds(&nfa);
        let mut stream = CharStream::from_string(String::from("a"));
        assert!(nfa.is_match(&mut stream));
    }

    #[test]
    pub fn test_nonsinglechar_rejects() {
        let nfa = unit('a');
        test_within_bounds(&nfa);
        let mut stream = CharStream::from_string(String::from("aa"));
        assert!(!nfa.is_match(&mut stream));
        stream = CharStream::from_string(String::from(""));
        assert!(!nfa.is_match(&mut stream));
    }

    #[test]
    pub fn test_times() {
        let nfa = times(&unit('a'), &unit('b'));
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
        let nfa = plus(&times(&unit('a'), &unit('b')), &times(&unit('c'), &unit('d')));
        test_within_bounds(&nfa);
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
        let nfa2 = times(&star(&times(&unit('a'), &unit('b'))), &unit('c'));
        println!("NFA2 is {:?}", nfa2);
    }

    #[test]
    pub fn test_star_with_plus_and_times() {
        let nfa = times(&star(&plus(&unit('a'), &unit('b'))), &star(&unit('c')));
        let nfa2 = star(&times(&unit('a'), &unit('b')));
        println!("NFA (ab)* is {:?}", nfa2);
        println!("NFA (ab)*c is {:?}", times(&nfa2, &unit('c')));
        print!("The nfa is {:?}", nfa);
        test_within_bounds(&nfa);
        test_within_bounds(&nfa2);
        let mut stream1 = CharStream::from_string(String::from("a"));
        let mut stream2 = CharStream::from_string(String::from("abababbbaba"));
        let mut stream3 = CharStream::from_string(String::from("abababbbabaccc"));
        let mut stream4 = CharStream::from_string(String::from("ababaaaababbaccbc"));
        assert!(nfa.is_match(&mut stream1));
        assert!(nfa.is_match(&mut stream2));
        assert!(nfa.is_match(&mut stream3));
        assert!(!nfa.is_match(&mut stream4));
    }
}



