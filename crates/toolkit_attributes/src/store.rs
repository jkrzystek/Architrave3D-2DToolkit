//! Top-level container holding one [`AttributeSet`] per [`Domain`].
//!
//! This is the unit a procedural tool or a mesh carries alongside its geometry:
//! "give me the `Cd` color on points, the `material` id on primitives, the
//! `bounds` on detail". Each domain keeps its own element count.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::set::AttributeSet;
use crate::types::{Attribute, AttributeType, Domain};

/// A bundle of attribute sets keyed by domain.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AttributeStore {
    sets: BTreeMap<Domain, AttributeSet>,
}

impl AttributeStore {
    /// An empty store with no domains populated.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the set for a domain, if it exists.
    pub fn domain(&self, domain: Domain) -> Option<&AttributeSet> {
        self.sets.get(&domain)
    }

    /// Get (creating an empty, zero-count set if absent) the set for a domain.
    pub fn domain_mut(&mut self, domain: Domain) -> &mut AttributeSet {
        self.sets.entry(domain).or_insert_with(|| AttributeSet::new(0))
    }

    /// Set the element count for a domain, creating it if needed.
    pub fn set_domain_count(&mut self, domain: Domain, count: usize) {
        self.domain_mut(domain).resize(count);
    }

    /// Element count for a domain (0 if the domain is absent).
    pub fn domain_count(&self, domain: Domain) -> usize {
        self.sets.get(&domain).map(|s| s.count()).unwrap_or(0)
    }

    /// Create an attribute on a domain. The domain is created if absent (with
    /// the given `count` as its element count); otherwise the existing count is
    /// used and `count` is ignored.
    pub fn create(
        &mut self,
        domain: Domain,
        count: usize,
        name: impl Into<String>,
        ty: AttributeType,
    ) -> &mut Attribute {
        let set = self.sets.entry(domain).or_insert_with(|| AttributeSet::new(count));
        set.create(name, ty)
    }

    /// Look up an attribute on a domain.
    pub fn get(&self, domain: Domain, name: &str) -> Option<&Attribute> {
        self.sets.get(&domain).and_then(|s| s.get(name))
    }

    /// Look up an attribute mutably.
    pub fn get_mut(&mut self, domain: Domain, name: &str) -> Option<&mut Attribute> {
        self.sets.get_mut(&domain).and_then(|s| s.get_mut(name))
    }

    /// Whether a domain has the named attribute.
    pub fn contains(&self, domain: Domain, name: &str) -> bool {
        self.sets.get(&domain).map(|s| s.contains(name)).unwrap_or(false)
    }

    /// Remove an attribute, returning it if present.
    pub fn remove(&mut self, domain: Domain, name: &str) -> Option<Attribute> {
        self.sets.get_mut(&domain).and_then(|s| s.remove(name))
    }

    /// Iterate the populated domains.
    pub fn domains(&self) -> impl Iterator<Item = Domain> + '_ {
        self.sets.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn create_and_read_across_domains() {
        let mut store = AttributeStore::new();
        store.create(Domain::Point, 4, "P", AttributeType::Vec3).set_vec3(0, Vec3::Y);
        store.create(Domain::Primitive, 2, "material", AttributeType::Int);

        assert_eq!(store.domain_count(Domain::Point), 4);
        assert_eq!(store.domain_count(Domain::Primitive), 2);
        assert_eq!(
            store.get(Domain::Point, "P").unwrap().get_vec3(0),
            Some(Vec3::Y)
        );
    }

    #[test]
    fn second_create_uses_existing_count() {
        let mut store = AttributeStore::new();
        store.create(Domain::Point, 8, "P", AttributeType::Vec3);
        // count arg ignored since domain exists.
        store.create(Domain::Point, 999, "Cd", AttributeType::Color);
        assert_eq!(store.get(Domain::Point, "Cd").unwrap().len(), 8);
    }

    #[test]
    fn set_domain_count_resizes() {
        let mut store = AttributeStore::new();
        store.create(Domain::Point, 2, "P", AttributeType::Vec3);
        store.set_domain_count(Domain::Point, 10);
        assert_eq!(store.get(Domain::Point, "P").unwrap().len(), 10);
    }

    #[test]
    fn missing_domain_reads_as_empty() {
        let store = AttributeStore::new();
        assert_eq!(store.domain_count(Domain::Face), 0);
        assert!(store.get(Domain::Face, "x").is_none());
        assert!(!store.contains(Domain::Face, "x"));
    }

    #[test]
    fn remove_works() {
        let mut store = AttributeStore::new();
        store.create(Domain::Detail, 1, "seed", AttributeType::Int);
        assert!(store.remove(Domain::Detail, "seed").is_some());
        assert!(!store.contains(Domain::Detail, "seed"));
    }

    #[test]
    fn serde_roundtrip() {
        let mut store = AttributeStore::new();
        store.create(Domain::Point, 3, "P", AttributeType::Vec3);
        store.create(Domain::Detail, 1, "seed", AttributeType::Int);
        let json = serde_json::to_string(&store).unwrap();
        let back: AttributeStore = serde_json::from_str(&json).unwrap();
        assert_eq!(store, back);
    }
}
