//! L-system string rewriting: an axiom plus per-symbol production rules,
//! expanded over a number of iterations. Supports stochastic rules (weighted
//! alternatives chosen with a seeded RNG).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use toolkit_rng::Rng;

/// One possible replacement for a symbol, with a relative `weight`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Production {
    pub weight: f32,
    pub successor: String,
}

/// An L-system: a starting `axiom` and a map from a symbol to its possible
/// replacements. Symbols without a rule are copied unchanged (they become
/// turtle commands or constants).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LSystem {
    pub axiom: String,
    rules: HashMap<char, Vec<Production>>,
}

impl LSystem {
    pub fn new(axiom: impl Into<String>) -> Self {
        Self {
            axiom: axiom.into(),
            rules: HashMap::new(),
        }
    }

    /// Add a deterministic rule (weight 1). Multiple rules for the same symbol
    /// become stochastic alternatives.
    pub fn rule(mut self, predecessor: char, successor: impl Into<String>) -> Self {
        self.add_weighted(predecessor, 1.0, successor);
        self
    }

    /// Add a weighted alternative for a symbol.
    pub fn weighted_rule(mut self, predecessor: char, weight: f32, successor: impl Into<String>) -> Self {
        self.add_weighted(predecessor, weight, successor);
        self
    }

    fn add_weighted(&mut self, predecessor: char, weight: f32, successor: impl Into<String>) {
        self.rules.entry(predecessor).or_default().push(Production {
            weight,
            successor: successor.into(),
        });
    }

    /// Deterministically expand for `iterations`, always taking the
    /// highest-weight (first, on ties) production for each symbol.
    pub fn expand(&self, iterations: u32) -> String {
        self.expand_with(iterations, &mut |prods| {
            prods
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.weight.total_cmp(&b.weight))
                .map(|(i, _)| i)
                .unwrap_or(0)
        })
    }

    /// Expand for `iterations`, choosing among alternatives with `rng`
    /// (weighted). Reproducible for a given seed.
    pub fn expand_stochastic(&self, iterations: u32, rng: &mut Rng) -> String {
        self.expand_with(iterations, &mut |prods| weighted_index(prods, rng))
    }

    /// Core rewrite loop; `choose` selects which production to apply.
    fn expand_with(&self, iterations: u32, choose: &mut dyn FnMut(&[Production]) -> usize) -> String {
        let mut current = self.axiom.clone();
        for _ in 0..iterations {
            let mut next = String::with_capacity(current.len() * 2);
            for ch in current.chars() {
                match self.rules.get(&ch) {
                    Some(prods) if !prods.is_empty() => {
                        let idx = choose(prods).min(prods.len() - 1);
                        next.push_str(&prods[idx].successor);
                    }
                    _ => next.push(ch),
                }
            }
            current = next;
        }
        current
    }
}

fn weighted_index(prods: &[Production], rng: &mut Rng) -> usize {
    let total: f32 = prods.iter().map(|p| p.weight.max(0.0)).sum();
    if total <= 0.0 {
        return 0;
    }
    let mut pick = rng.range_f32(0.0, total);
    for (i, p) in prods.iter().enumerate() {
        pick -= p.weight.max(0.0);
        if pick <= 0.0 {
            return i;
        }
    }
    prods.len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algae_classic_growth() {
        // Lindenmayer's algae: A -> AB, B -> A.
        let sys = LSystem::new("A").rule('A', "AB").rule('B', "A");
        assert_eq!(sys.expand(0), "A");
        assert_eq!(sys.expand(1), "AB");
        assert_eq!(sys.expand(2), "ABA");
        assert_eq!(sys.expand(3), "ABAAB");
        assert_eq!(sys.expand(4), "ABAABABA");
        // Length follows the Fibonacci sequence.
        assert_eq!(sys.expand(5).len(), 13);
    }

    #[test]
    fn symbols_without_rules_pass_through() {
        let sys = LSystem::new("F+F").rule('F', "FF");
        assert_eq!(sys.expand(1), "FF+FF");
    }

    #[test]
    fn stochastic_is_reproducible() {
        let sys = LSystem::new("X")
            .weighted_rule('X', 1.0, "a")
            .weighted_rule('X', 1.0, "b");
        let mut r1 = Rng::seed_from_u64(99);
        let mut r2 = Rng::seed_from_u64(99);
        assert_eq!(sys.expand_stochastic(5, &mut r1), sys.expand_stochastic(5, &mut r2));
    }

    #[test]
    fn stochastic_can_pick_either_branch() {
        let sys = LSystem::new("X")
            .weighted_rule('X', 1.0, "a")
            .weighted_rule('X', 1.0, "b");
        // Over many seeds we should see both outcomes.
        let mut seen_a = false;
        let mut seen_b = false;
        for seed in 0..32 {
            let mut rng = Rng::seed_from_u64(seed);
            match sys.expand_stochastic(1, &mut rng).as_str() {
                "a" => seen_a = true,
                "b" => seen_b = true,
                _ => {}
            }
        }
        assert!(seen_a && seen_b);
    }

    #[test]
    fn serde_roundtrip() {
        let sys = LSystem::new("A").rule('A', "AB").rule('B', "A");
        let json = serde_json::to_string(&sys).unwrap();
        let back: LSystem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.expand(3), "ABAAB");
    }
}
