//! # Host events
//!
//! Host events are a pattern for emitting Rust-side events into JavaScript
//! callbacks. Define an enum whose variants carry the event's arguments,
//! derive [`HostEvent`] on it, and the macro will generate the plumbing that
//! converts each variant into a JS callback name plus a list of [`JsiValue`]
//! arguments.

use crate::{JsiValue, RuntimeHandle};
use std::str::FromStr;

/// A key that identifies an event type. Implemented automatically by
/// `#[derive(HostEvent)]` as a companion `<EnumName>Key` enum.
pub trait HostEventKey:
    Copy
    + Clone
    + Eq
    + PartialEq
    + Ord
    + PartialOrd
    + std::hash::Hash
    + std::fmt::Debug
    + FromStr<Err = anyhow::Error>
{
}

/// An event that can be converted into a JS callback name and a list of
/// arguments. Implemented automatically by `#[derive(HostEvent)]`.
pub trait HostEvent<'rt>: Sized {
    type Key: HostEventKey;

    fn key(&self) -> Self::Key;

    fn args(self, rt: &mut RuntimeHandle<'rt>) -> Vec<JsiValue<'rt>>;
}
