//! AI chat components for [Topcoat](https://github.com/tokio-rs/topcoat), shipped as a
//! `topcoat ui` component registry.
//!
//! Add this crate as a dependency, then install components into your app:
//!
//! ```sh
//! topcoat ui add message --registry top-prompt
//! ```
//!
//! The crate itself has no runtime code. `registry.toml` names the component sources
//! under `src/components/`, which `topcoat ui add` copies into your project together
//! with the built-in Topcoat components they use (`button`, `tooltip`, `badge`, ...).
//!
//! The sources refer to those built-ins as sibling modules (`super::button`), so they
//! only compile once installed next to them. The docs site in this repository (`site/`)
//! installs every component and serves as their compile and test check.
