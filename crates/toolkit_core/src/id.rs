use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_unique_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(u64);

        impl $name {
            pub fn new() -> Self {
                Self(next_unique_id())
            }

            pub fn from_raw(raw: u64) -> Self {
                Self(raw)
            }

            pub fn raw(self) -> u64 {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id!(LayerId);
define_id!(TextureId);
define_id!(NodeId);
define_id!(MeshId);
define_id!(MaterialId);
define_id!(ViewportId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let a = LayerId::new();
        let b = LayerId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn id_roundtrip_raw() {
        let id = TextureId::from_raw(42);
        assert_eq!(id.raw(), 42);
    }

    #[test]
    fn different_id_types_independent() {
        let layer = LayerId::from_raw(1);
        let texture = TextureId::from_raw(1);
        assert_eq!(layer.raw(), texture.raw());
    }
}
