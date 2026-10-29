# jsi-rs

Rust bindings and tooling for writing React Native JSI modules in Rust. Workspace crates: `jsi-sys` (cxx FFI), `jsi` (safe-ish wrapper), `jsi-macros` (`#[host_object]`), `jsi-turbo` / `jsi-turbo-macros` (Rust TurboModule framework), `jsi-tests` (runs a real Hermes runtime — needs `git submodule update --init` and Ninja; Hermes is built once by the test build script).

jsi-turbo 的现状、待办任务(可认领)与设计决策记录见 `jsi-turbo/ROADMAP.md`。

## Git conventions

- 提交信息不要添加 `Co-Authored-By: Claude Code` 或任何 AI 署名尾行(no attribution trailer)。
- 新功能在从 main 切出的 `feature/<name>` 分支上开发。
