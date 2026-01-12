//! Cache storage.

mod memory;
mod traits;

pub use memory::MemoryCache;
pub use traits::{CacheStorage, CacheStorageExt};
