# `jsi-rs`

This library makes it possible to write React Native JSI modules in Rust.

For an example, check out the `example` folder.

## Getting Started

1. Clone this repo
2. Run `git submodule init`
3. Run `git submodule update`, this will ensure that all the vendor dependencies are cloned locally
4. Make sure you have Ninja installed locally, which is necessary for building Hermes. You can find instructions [here](https://github.com/ninja-build/ninja/wiki/Pre-built-Ninja-packages). On macOS, you can install it with `brew install ninja`
5. Run the tests: `cargo test -p jsi-tests` (runs against a real Hermes runtime)
6. To try a complete React Native app, follow [`example/README.md`](example/README.md)

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

## Contributing

I wrote this code in winter 2022 as part of another project. A few months later,
I have decided to release it to the world. However, I'm not planning to maintain
it unless I encounter another project that requires it, so for now, the code is
given to you as-is. Feel free to contribute PRs that would improve the API or
stability of the library.

## Safety

Right now, this library is quite `unsafe`.

## Copyright / license

Copyright Ibiyemi Abiodun. MIT License.
