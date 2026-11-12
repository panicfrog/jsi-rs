# Changelog

## 0.88.5 (2026-11-12)

An npm-package release. Both published-artifact changes live in
`react-native-rust-turbo`'s build scaffolding (iOS podspec, Android
CMake); the crates carry no functional changes — their patch bump just
keeps every artifact on one version.

### react-native-rust-turbo: Xcode no longer silently skips the Rust build

The `RustTurboBuild` script phase declared `:output_files` with no
`:input_files`, so Xcode's dependency analysis saw the archive from a
previous build and skipped the phase entirely: cargo never ran, and the
app silently linked the stale `librust-app.a`. Source changes to the
module packages stopped reaching the binary — observed downstream as
the app running days-old code with only `ld` version warnings in the
build log.

The declaration is dropped; the phase now runs on every build. A
no-change build costs cargo's incremental fingerprint check (seconds) —
the same trade RN's own "Bundle React Native code and images" phase
makes. (Enumerating `:input_files` instead would require listing every
Rust/Swift source across the module packages' boundaries, and missing
one file re-opens the exact silent-staleness hole this closes.)

### react-native-rust-turbo: Android links `liblog`

`librust_app.a` is a static archive and cannot carry its own linker
dependencies, so the platform libraries the Rust side needs must be
listed at the `.so` link in the template `CMakeLists.txt`. `log`
(android_logger's backend) is now linked there; without it the link
fails with `undefined symbol: __android_log_buf_write`.

### Development-side (not in the published artifacts)

- vendored Hermes moved to the exact tag RN 0.88.1 builds its prebuilt
  `.so` from (`hermes-v260318099.0.5`), so host tests run the engine
  apps actually ship; `build-hermes.sh` takes a build type, and the
  embedder README documents the `setImmediate` requirement newer
  Hermes' built-in Promise machinery has.
- Ground-truth tests for the lifetime claim every `NativeState`
  consumer stands on: a plain payload and a TurboObject instance (the
  ModuleHolder/Arc chain, method-call path included) are both reclaimed
  by `gc()` with their Rust `Drop` run.

## 0.88.4 (2026-11-05)

Two hardening fixes, both surfaced by monet-rn — the first TurboStruct
consumer with cross-referencing structs and multi-word field names.

### jsi: `deserialize_any` handles booleans and stops panicking

`JsiDeserializer::deserialize_any` dispatched on the JS value's type but
had no boolean arm, so any boolean reaching it hit `unreachable!()`. Host
functions sit behind an `extern "C"` boundary, so the panic unwound into
cxx's `prevent_unwind` and **aborted the whole app**.

Booleans reach that path through serde's `IgnoredAny`: a struct field
whose key does not match the Rust field name (a TypeScript consumer
sending `followRedirects` where serde expects `follow_redirects`, say)
has its value skipped via `IgnoredAny`, which delegates to
`deserialize_any` regardless of the value's type.

- `is_bool()` now routes to `deserialize_bool`.
- The remaining fallthrough (functions, ArrayBuffers, BigInts, …) returns
  a conversion error instead of `unreachable!()` — that path also serves
  `IgnoredAny`, where the value's type is arbitrary.

### jsi-turbo-macros: struct files import their siblings correctly

The TurboStruct/TurboObject interfaces are written into
`generated/rust-turbo/structs/`, but the `import type` lines they emit
used the module file's perspective (`'./structs/X'`). A struct
referencing another struct (monet-rn's `FetchOptions` → `HeaderKV`)
therefore produced `structs/FetchOptions.ts` importing
`'./structs/HeaderKV'` — a path that does not exist, failing `tsc` with
TS2307. Both emitters now rewrite the `./structs/` prefix to `./` for
files written inside that directory.

### Note for module authors

The generated TypeScript spells struct fields in camelCase (`timeoutMs`).
A Rust struct with snake_case fields must carry
`#[serde(rename_all = "camelCase")]`, or every multi-word field arrives
as an unknown key — and until this release, a boolean one of those
aborted the process. The generator could emit the attribute
automatically in a future release.
