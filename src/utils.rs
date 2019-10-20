use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::{HashMap, HashSet};
const ID_CHARS: [char; 62] = [
    'A', 'Z', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', 'Q', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L',
    'M', 'W', 'X', 'C', 'V', 'B', 'N', 'a', 'z', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'q', 's',
    'd', 'f', 'g', 'h', 'j', 'k', 'l', 'm', 'w', 'x', 'c', 'v', 'b', 'n', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9',
];
const ID_SIZE: usize = 5;

pub fn rand_id() -> String {
    let mut rng = thread_rng();
    let mut s = String::with_capacity(ID_SIZE);
    for _ in 0..ID_SIZE {
        s.push(*ID_CHARS.choose(&mut rng).unwrap());
    }
    s
}

pub fn pop_set<T: Clone + Eq + std::hash::Hash>(set: &mut HashSet<T>) -> T {
    let elt = set.iter().next().cloned().unwrap();
    set.take(&elt).unwrap()
}

pub fn time<F, K>(f: F) -> u128
where
    F: FnOnce() -> K,
{
    let start = std::time::Instant::now();
    f();
    start.elapsed().as_micros()
}
