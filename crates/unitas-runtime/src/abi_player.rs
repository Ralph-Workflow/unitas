//! The player host's runtime-side ABI.
//!
//! The managed player host used to publish the game's scraped HUD state
//! through this module. The host reports no game-specific state any more, so
//! the module carries no exports; the declaration in `lib.rs` outlives the
//! file's contents until that declaration is dropped.
