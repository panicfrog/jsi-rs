# `js-tests`

This crate contains tests for `jsi-rs`. The tests are run against
[Hermes](https://github.com/facebook/hermes), which is Facebook's reference
implementation of JavaScript for React Native.

## Building and running tests

Hermes is a Git submodule of this repository, so it should be cloned
automatically when you clone this repo. The initial build of this crate will
take a while because it builds Hermes first.

Once that build finishes, you can `cargo test` as normal.

## Bare-runtime requirements

Hermes' built-in `Promise` (the `InternalBytecode` machinery) defers
resolution through the `setImmediate` global, which the engine itself does
not provide — React Native hosts install it, a bare runtime does not. The
test harness (`tests/common.rs`) installs an inline
`setImmediate = fn => { fn(); }` for this reason: the promise reactions it
triggers still land on the microtask queue, so driving a runtime is still
just "run the task, then `drain_microtasks`".

Embedders wiring up their own Hermes runtime outside of React Native have
to provide `setImmediate` the same way (any callback-scheduling shape
works; running it inline is enough when microtasks are drained
afterwards) — async host methods otherwise reject with
`ReferenceError: Property 'setImmediate' doesn't exist`.
