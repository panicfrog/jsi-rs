# `jsi-rs`

Rust bindings and tooling for React Native's JSI, plus a Rust TurboModule
framework. Write native modules in Rust, install them into the JS runtime
through JSI, and call them from JavaScript — no bridge, with TypeScript
bindings generated from a single attribute macro.

> This project was originally forked from
> [laptou/jsi-rs](https://github.com/laptou/jsi-rs) (thank you
> [@laptou](https://github.com/laptou) for the original work!) and is now
> maintained as an independent repository.

## Getting Started

1. Clone this repo
2. Run `git submodule init`
3. Run `git submodule update`, this will ensure that all the vendor dependencies are cloned locally
4. Make sure you have Ninja installed locally, which is necessary for building Hermes. You can find instructions [here](https://github.com/ninja-build/ninja/wiki/Pre-built-Ninja-packages). On macOS, you can install it with `brew install ninja`
5. Run the tests: `cargo test -p jsi-tests` (runs against a real Hermes runtime)
6. To try a complete React Native app, follow [`example/README.md`](example/README.md)

## Writing TurboModules

To use `jsi-turbo` in your own app, follow the usage guide:
[`jsi-turbo/docs/usage.md`](jsi-turbo/docs/usage.md)
(中文版:[usage.zh-CN.md](jsi-turbo/docs/usage.zh-CN.md)). In short: install
the core package plus your module packages, and the platform build hooks
take care of the cargo build and linking — no manual copy steps.

The current status and roadmap of `jsi-turbo` live in
[`jsi-turbo/ROADMAP.md`](jsi-turbo/ROADMAP.md).

> NOTE: Make sure that you have not installed rust with homebrew on mac, use the `rustup` toolchain instead.

## Crates

jsi is the engine-agnostic interface that Hermes, JSC and other engines
implement, so the crates are layered to keep the core independent of React
Native:

| Crate | Depends on | Contents |
|---|---|---|
| `jsi-sys` | `jsi.h` only | Low-level FFI to jsi |
| `jsi` | `jsi-sys` | Safe API, `#[host_object]`, serde conversion, the `JsScheduler` trait for posting work to the JS thread |
| `jsi-rn` | `jsi` + React Native | React Native's `CallInvoker` (implements `JsScheduler`), Android `CallInvokerHolder`, `jsi_rn::init` |
| `jsi-turbo` | `jsi` (+ `jsi-rn` for async) | Rust TurboModule framework: `#[RustTurboModule("Name")]`, lazy `RustTurbo.get(name)` proxy, TypeScript bindings generation |
| `jsi-macros` | — | Proc macros re-exported by `jsi` |

Only depend on `jsi-rn` if you need React Native specific APIs such as the
`CallInvoker`; a module that only installs bindings needs just `jsi`.

### Build configuration

- `JSI_DIR` (env, `jsi-sys`): directory containing `jsi/jsi.h`. Defaults to the
  vendored React Native tree. jsi.h is the ABI contract between host and
  engine, so it must be the same version the engine was built against.
- `REACT_NATIVE_DIR` (env, `jsi-rn`): root of the `react-native` package.
  Defaults to the vendored React Native tree.
- `bundled-jsi` (feature of `jsi` / `jsi-sys`, on by default): compiles
  `jsi.cpp` into the Rust library. Turn it off (`default-features = false`)
  when the host already provides jsi, e.g. in a React Native app, so that the
  program contains a single copy of it.

### Migrating from `jsi::init(rt, call_invoker)`

`jsi::init` now only takes the runtime pointer. React Native modules that need
the `CallInvoker` should call `jsi_rn::init(rt, call_invoker)` instead, which
returns the same `(RuntimeHandle, CallInvoker)` pair as before.

## Versioning

Package versions track the React Native **line** they target: major.minor
matches RN (the current `main` tracks 0.88.x), because jsi.h must match
the engine build exactly; the patch number is our own release count on
that line. Older React Native versions are supported through long-lived
branches, frozen once stable:

| Branch | React Native | Status |
|---|---|---|
| `rn-0.73` | 0.73 | frozen |
| `rn-0.77.3` | 0.77.3 | frozen |
| `main` | 0.88.1 | active development |

When upgrading to a new React Native release, both the `react-native` and
`hermes` submodules must be switched so that their copies of `jsi.h` are
byte-identical.

## Contributing

This project is actively maintained. Bug reports, PRs and API discussions are
all welcome — see the claimable tasks in [`jsi-turbo/ROADMAP.md`](jsi-turbo/ROADMAP.md)
if you want a starting point. New features are developed on `feature/<name>`
branches cut from `main`.

## Safety

Right now, this library is quite `unsafe`.

## Copyright / license

Copyright Ibiyemi Abiodun. MIT License.
