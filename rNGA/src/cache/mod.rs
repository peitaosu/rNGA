//! Cache storage.

mod memory;
mod traits;

pub use memory::MemoryCache;
pub use traits::{get_json, set_json, CacheStorage, CacheStorageExt};
