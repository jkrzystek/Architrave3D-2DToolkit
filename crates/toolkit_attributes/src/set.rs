//! A set of attributes that all share one domain and element count.
//!
//! Every channel in a set stays the same length as the set's `count`, so callers
//! can index any attribute by the same element index. Creating a channel fills
//! it with defaults; resizing the set resizes every channel together.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::{Attribute, AttributeType};

/// All attributes bound to a single domain.
///
/// Uses a `BTreeMap` so iteration order is deterministic (handy for hashing,
/// diffing, and stable serialization).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AttributeSet {
    count: usize,
    attributes: BTreeMap<String, Attribute>,
}

impl AttributeSet {
    /// An empty set sized for `count` elements.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            attributes: BTreeMap::new(),
        }
    }

    /// Number of elements every channel is sized to.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Number of channels.
    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    /// Whether there are no channels.
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }

    /// Create (or replace) a channel of the given type, sized to `count` with
    /// default values. Returns a mutable reference to it.
    pub fn create(&mut self, name: impl Into<String>, ty: AttributeType) -> &mut Attribute {
        let name = name.into();
        let attr = Attribute::new(name.clone(), ty, self.count);
        self.attributes.entry(name.clone()).or_insert(attr);
        // Replace if the type differs from what was there.
        let slot = self.attributes.get_mut(&name).unwrap();
        if slot.ty() != ty || slot.len() != self.count {
            *slot = Attribute::new(name, ty, self.count);
        }
        slot
    }

    /// Insert a pre-built attribute, resizing it to match the set's count.
    pub fn insert(&mut self, mut attr: Attribute) {
        attr.resize(self.count);
        self.attributes.insert(attr.name.clone(), attr);
    }

    /// Whether a channel exists.
    pub fn contains(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    /// Look up a channel.
    pub fn get(&self, name: &str) -> Option<&Attribute> {
        self.attributes.get(name)
    }

    /// Look up a channel mutably.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Attribute> {
        self.attributes.get_mut(name)
    }

    /// Remove and return a channel.
    pub fn remove(&mut self, name: &str) -> Option<Attribute> {
        self.attributes.remove(name)
    }

    /// Channel names, in sorted order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.attributes.keys().map(|s| s.as_str())
    }

    /// Iterate all channels.
    pub fn iter(&self) -> impl Iterator<Item = &Attribute> {
        self.attributes.values()
    }

    /// Resize the set, resizing every channel to match.
    pub fn resize(&mut self, count: usize) {
        self.count = count;
        for attr in self.attributes.values_mut() {
            attr.resize(count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AttributeType;
    use glam::Vec3;

    #[test]
    fn create_sizes_to_count() {
        let mut set = AttributeSet::new(8);
        let p = set.create("P", AttributeType::Vec3);
        assert_eq!(p.len(), 8);
        assert!(set.contains("P"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn create_twice_keeps_data_when_same_type() {
        let mut set = AttributeSet::new(4);
        set.create("P", AttributeType::Vec3).set_vec3(0, Vec3::X);
        // Re-create with same type should not wipe existing values.
        let p = set.create("P", AttributeType::Vec3);
        assert_eq!(p.get_vec3(0), Some(Vec3::X));
    }

    #[test]
    fn create_replaces_on_type_change() {
        let mut set = AttributeSet::new(4);
        set.create("x", AttributeType::Float);
        let x = set.create("x", AttributeType::Int);
        assert_eq!(x.ty(), AttributeType::Int);
    }

    #[test]
    fn insert_resizes_to_set() {
        let mut set = AttributeSet::new(5);
        let attr = Attribute::new("w", AttributeType::Float, 2);
        set.insert(attr);
        assert_eq!(set.get("w").unwrap().len(), 5);
    }

    #[test]
    fn resize_resizes_all_channels() {
        let mut set = AttributeSet::new(2);
        set.create("P", AttributeType::Vec3);
        set.create("w", AttributeType::Float);
        set.resize(6);
        assert_eq!(set.count(), 6);
        assert_eq!(set.get("P").unwrap().len(), 6);
        assert_eq!(set.get("w").unwrap().len(), 6);
    }

    #[test]
    fn remove_drops_channel() {
        let mut set = AttributeSet::new(1);
        set.create("tmp", AttributeType::Bool);
        assert!(set.remove("tmp").is_some());
        assert!(!set.contains("tmp"));
    }

    #[test]
    fn serde_roundtrip() {
        let mut set = AttributeSet::new(3);
        set.create("P", AttributeType::Vec3);
        let json = serde_json::to_string(&set).unwrap();
        let back: AttributeSet = serde_json::from_str(&json).unwrap();
        assert_eq!(set, back);
    }
}
