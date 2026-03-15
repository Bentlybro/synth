use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Generate a cache key from any hashable values
pub fn cache_key<T: Hash + ?Sized>(value: &T) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Generate a cache key from multiple values
pub fn cache_key_multi(values: &[&str]) -> String {
    let combined = values.join(":");
    cache_key(&combined)
}
