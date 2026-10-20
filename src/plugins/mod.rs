//! Tool plugins: fleet's seam to a separate program that owns its own
//! home, its own state, and its own actions.
//!
//! The channel service carries work from outside, and a lane is a model
//! service fleet launches and probes. A tool plugin is neither. It is a
//! program the owner already runs — with its own repository, its own
//! pipeline, and its own write vocabulary — and fleet's job is to
//! surface it without becoming a second implementation of it. Second
//! brain is the first: fleet reads the state files the tool publishes
//! and forwards a refresh as the tool's own `POST /api/run`.
//!
//! There is deliberately no trait yet. The registry is a module, and the
//! shape a second tool would share with this one is not known until
//! there is a second tool; a trait written now would be a guess with a
//! name.

pub mod second_brain;
