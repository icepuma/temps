//! Compiles and runs the Rust code blocks of the workspace `README.md` as doctests, so the
//! snippets users copy cannot drift from the API. `just doc-test` and CI run them.
//!
//! The check lives in this unpublished crate rather than in `temps` for two reasons:
//!
//! - its only dependency is `temps`, as for a user following the README, so a snippet that
//!   imports anything else (such as `temps_core`) fails to compile here;
//! - the README sits outside every published crate's directory, so an `include_str!` of it
//!   in a published crate breaks `cargo test` on that crate's `.crate` tarball.

#[cfg(doctest)]
#[doc = include_str!("../../README.md")]
pub struct ReadmeDoctests;
