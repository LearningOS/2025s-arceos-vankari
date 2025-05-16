#[cfg(feature = "alloc")]  
mod hash_map;  

#[cfg(feature = "alloc")]  
pub use hash_map::HashMap;