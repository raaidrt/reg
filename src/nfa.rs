pub mod node;
use char_stream::CharStream;
use node::Node;
use std::collections::{HashMap, HashSet};
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};

#[derive(Debug)]
pub struct NFA {
    states: usize,
    starting: HashSet<Node>,
    delta: HashMap<(Node, char), HashSet<Node>>,
    finished: HashSet<Node>,
}

fn recfn(nfa: &'static NFA, seen_configs: & mut Arc<HashSet<(Node, usize)>>, num_threads: &Arc<Mutex<usize>>, hashset_mutex: &Arc<Mutex<()>>, rx: Receiver<bool>, tx: Sender<bool>, node: Node, index: usize, string: &'static str) {
    let send_and_update_mutex = |to_send: bool| {
        let mut num_threads = num_threads.lock().unwrap();
        *num_threads -= 1;
        tx.send(to_send).unwrap();
    };
    if index >= string.len() && !nfa.finished.contains(&node) {
        send_and_update_mutex(false);
    } else if nfa.finished.contains(&node) {
        send_and_update_mutex(true);
    } else if seen_configs.contains(&(node, index)) {
        send_and_update_mutex(false);
    } else {
        {
            let _ = hashset_mutex.lock().unwrap();
            (*Arc::make_mut(seen_configs)).insert((node.clone(), index));
        }
        if let Some(set) = nfa.delta.get(&(node, string.as_bytes()[index] as char)) {
            for &node in set {
                let num_threads = Arc::clone(num_threads);
                let hashset_mutex = Arc::clone(hashset_mutex);
                let seen_configs = Arc::clone(seen_configs);
                thread::spawn(move || {
                    recfn(nfa, &mut seen_configs, &num_threads, &hashset_mutex, rx, tx.clone(), node.clone(), index + 1, string);
                });
            }
        }
    }
}
impl NFA {
    pub fn is_match(&self, string: &String) -> bool {
        let seen_configs: Arc<HashSet<(Node, u128)>> = Arc::new([].into());
        let num_threads = Arc::new(Mutex::new(self.starting.len()));
        let hashset_mutex = Arc::new(Mutex::new(()));
        let (tx, rx) = channel();
        let mut works = false;
        
        
        for &Node(n) in self.starting.iter() {
            let (num_threads, tx) = (Arc::clone(&num_threads), tx.clone());
            
            

            thread::spawn(move || {
                let (num_threads, tx) = (Arc::clone(&num_threads), tx.clone());
                let mut num_threads = num_threads.lock().unwrap();
                *num_threads -= 1;
            });
        }
        
        while {
            let num = (*num_threads).lock().unwrap();
            *num > 0
        } {
            works = works || rx.recv().unwrap();
        }

        let mut nodes: HashSet<Node> = self.starting.clone();
        for ch in CharStream::from(string) {
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
    let states = first.states + second.states;
    let increase = |&node: &Node| -> Node {
        let Node(n) = node;
        return Node(n + first.states);
    };
    let mut starting = first.starting.clone();
    if first
        .starting
        .iter()
        .any(|&node| first.finished.contains(&node))
    {
        starting = starting
            .union(&second.starting.iter().map(increase).collect())
            .copied()
            .collect();
    }
    // any nodes mapping to a first.finished state should map to second.starting states as well
    let mut delta = first.delta.clone();
    let finished: HashSet<Node> = second.finished.clone().iter().map(increase).collect();
    let second_starting: HashSet<Node> = second.starting.clone().iter().map(increase).collect();
    for (&(Node(n), ch), set) in first.delta.iter() {
        let mut new_set: HashSet<Node> = set.clone();
        if set.iter().any(|&node| first.finished.contains(&node)) {
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
    let mut delta = nfa.delta.clone();
    for (&(Node(n), ch), set) in nfa.delta.iter() {
        let mut new_set = set.clone();
        if set.iter().any(|&node| nfa.finished.contains(&node)) {
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
        let stream = String::from("");
        assert!(nfa.is_match(&stream));
    }

    #[test]
    pub fn test_nonempty_rejects() {
        let nfa = empty();
        test_within_bounds(&nfa);
        let stream = String::from("a");
        assert!(!nfa.is_match(&stream));
    }

    #[test]
    pub fn test_single_char() {
        let nfa = unit('a');
        test_within_bounds(&nfa);
        let stream = String::from("a");
        assert!(nfa.is_match(&stream));
    }

    #[test]
    pub fn test_nonsinglechar_rejects() {
        let nfa = unit('a');
        test_within_bounds(&nfa);
        let stream = String::from("aa");
        assert!(!nfa.is_match(&stream));
        let stream = String::from("");
        assert!(!nfa.is_match(&stream));
    }

    #[test]
    pub fn test_times() {
        let nfa = times(&unit('a'), &unit('b'));
        test_within_bounds(&nfa);
        println!("NFA ab is {:?}", nfa);
        let mut stream = String::from("ab");
        assert!(nfa.is_match(&stream));
        stream = String::from("ba");
        assert!(!nfa.is_match(&stream));
        let another_nfa = times(&empty(), &nfa);
        stream = String::from("ab");
        println!("Another nfa is {:?}", another_nfa);
        assert!(another_nfa.is_match(&mut stream));
    }

    #[test]
    pub fn test_plus() {
        let nfa = plus(
            &times(&unit('a'), &unit('b')),
            &times(&unit('c'), &unit('d')),
        );
        let stream1 = String::from("ab");
        let stream2 = String::from("cd");
        let stream3 = String::from("ac");
        let stream4 = String::from("cb");
        println!("The nfa is {:?}", nfa);
        assert!(nfa.is_match(&stream1));
        assert!(nfa.is_match(&stream2));
        assert!(!nfa.is_match(&stream3));
        assert!(!nfa.is_match(&stream4));
    }

    #[test]
    pub fn test_star_simple() {
        let nfa = star(&unit('a'));
        test_within_bounds(&nfa);
        println!("NFA a* is {:?}", nfa);
        let stream = String::from("");
        assert!(nfa.is_match(&stream));
        let stream = String::from("a");
        assert!(nfa.is_match(&stream));
        let stream = String::from("aa");
        assert!(nfa.is_match(&stream));
        let stream = String::from("aba");
        assert!(!nfa.is_match(&stream));
        let nfa2 = times(&star(&times(&unit('a'), &unit('b'))), &unit('c'));
        println!("NFA2 is {:?}", nfa2);
    }

    #[test]
    pub fn test_star_with_plus_and_times() {
        let nfa = times(&star(&plus(&unit('a'), &unit('b'))), &star(&unit('c')));
        let nfa2 = star(&times(&unit('a'), &unit('b')));
        println!("NFA (ab)* is {:?}", nfa2);
        println!("NFA (ab)*c is {:?}", times(&nfa2, &unit('c')));
        let stream = String::from("ab");
        assert!(!times(&nfa2, &unit('c')).is_match(&stream));
        let stream = String::from("ababababc");
        assert!(times(&nfa2, &unit('c')).is_match(&stream));
        println!("The nfa is {:?}", nfa);
        test_within_bounds(&nfa);
        test_within_bounds(&nfa2);
        let stream1 = String::from("a");
        let stream2 = String::from("abababbbaba");
        let stream3 = String::from("abababbbabaccc");
        let stream4 = String::from("ababaaaababbaccbc");
        assert!(nfa.is_match(&stream1));
        assert!(nfa.is_match(&stream2));
        assert!(nfa.is_match(&stream3));
        assert!(!nfa.is_match(&stream4));
    }
}
