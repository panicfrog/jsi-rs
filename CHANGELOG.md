# Changelog

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
