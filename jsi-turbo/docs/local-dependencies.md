# 本地依赖接入(在 jsi-rs 仓库外使用 jsi-turbo)

不发布 crates.io 也能用:你的工程以 **path 依赖**引用 jsi-rs 的 crate,
并用**你自己 app 的 React Native** 编译。核心是三个环境变量 +
`bundled-jsi` 关闭。以下步骤已在仓库外的全新 RN 0.88.1 app 上完整验证
(iOS 模拟器端到端:方法/getter/async Promise/诊断 API)。

## 1. Rust crate(放进你的 app)

`<app>/native/rust-app/`,一个 crate 搞定聚合库 + 模块:

```toml
# Cargo.toml
[lib]
crate-type = ["staticlib"]

[dependencies]
# default-features = false 关闭 bundled-jsi:RN 自带 jsi 符号,
# Rust 侧不编第二份(单一实现,无 ODR 风险)
jsi      = { path = "<jsi-rs 路径>/jsi", default-features = false, features = ["macros", "serde"] }
jsi-sys  = { path = "<jsi-rs 路径>/jsi-sys" }
jsi-rn   = { path = "<jsi-rs 路径>/jsi-rn" }
jsi-turbo = { path = "<jsi-rs 路径>/jsi-turbo" }
linkme = "0.3"
anyhow = "1.0"
cxx = "1.0"
serde = { version = "1.0", features = ["derive"] }

[build-dependencies]
cxx-build = "1.0"
```

```toml
# .cargo/config.toml —— 整个「本地依赖」的核心接线。
# 相对路径基于本文件所在目录,可提交、可移植;
# 所有 cargo 入口(命令行、xcodebuild 脚本、gradle task)自动生效。
[env]
JSI_DIR = { value = "../../node_modules/react-native/ReactCommon/jsi", relative = true }
REACT_NATIVE_DIR = { value = "../../node_modules/react-native", relative = true }
# Android 构建还需 fbjni 头(不在 react-native npm 包里):
# FBJNI_DIR = { value = "<fbjni headers 目录>", relative = true }
```

`src/lib.rs` 骨架(模块 + 安装入口同 crate;多模块时模块拆 crate、
此处加依赖并补 FORCE_LINK 数组):

```rust
use std::pin::Pin;
use jsi::RuntimeHandle;
use jsi_turbo::RustTurboModule;

#[cxx::bridge]
mod bridge {
    unsafe extern "C++" {
        include!("wrapper.h");
        include!("rn_wrapper.h");
        #[namespace = "facebook::jsi"]
        type Runtime = jsi_sys::Runtime;
        #[namespace = "facebook::react"]
        type CallInvoker = jsi_rn::sys::CallInvoker;
    }
    extern "Rust" {
        fn rust_turbo_install(rt: Pin<&mut Runtime>, invoker: SharedPtr<CallInvoker>) -> bool;
    }
}

// iOS 必需:防 staticlib 裁剪(每个模块一个锚)
#[used]
static FORCE_LINK: [extern "C" fn(); 1] = [__jsi_turbo_force_your_module];

fn rust_turbo_install(rt: Pin<&mut jsi_sys::Runtime>, invoker: cxx::SharedPtr<jsi_rn::sys::CallInvoker>) -> bool {
    let rt = unsafe { Pin::into_inner_unchecked(rt) };
    let raw = rt as *mut jsi_sys::Runtime;
    let (mut rt, invoker) = jsi_rn::init(raw, invoker);
    let dispatcher = jsi_turbo::Dispatcher::call_invoker(invoker, raw);
    jsi_turbo::install(&mut rt, dispatcher).is_ok()
}

#[derive(Default)]
pub struct YourModule;

#[RustTurboModule("YourModule")]
impl YourModule {
    pub fn add(&self, _rt: &mut RuntimeHandle, a: f64, b: f64) -> anyhow::Result<f64> { Ok(a + b) }
}
```

`build.rs`:`cxx_build::bridges(["src/lib.rs"]).flag_if_supported("-std=c++17").compile("rust-app");`

## 2. 平台接线(照抄 `jsi-turbo/carrier/`)

iOS:拷 `RustTurboModule.mm`,建 `RustTurbo.podspec`(`vendored_libraries
= "librust-app.a"`,别忘 `s.authors`/`s.homepage` 必填),Podfile 加
`pod 'RustTurbo', :path => '../native/rust-app'`。

Android:拷三件套 + CMake snippet,链 `librust_app.a`(staticlib,别用
cdylib)。ABI 见 `gradle.properties` 的 `reactNativeArchitectures`。

## 3. 构建与使用

```bash
cd native/rust-app
cargo build --release --target aarch64-apple-ios-sim
cp target/aarch64-apple-ios-sim/release/librust_app.a .   # podspec 旁
```

TS 直接 import 宏生成的绑定(`generated/rust-turbo/YourModule.ts`),
其余平台流程与你 app 原生一致。

## 常见报错 → 原因

| 报错 | 原因 |
|---|---|
| `jsi-sys: could not find jsi/jsi.h` | 环境变量没生效——检查 `.cargo/config.toml` 位置(crate 根下)与相对路径层级 |
| `The RustTurbo pod failed to validate` | podspec 缺 `authors`/`homepage` 必填属性 |
| `RustTurbo.get('X')` 报未注册 | iOS:模块没进 `FORCE_LINK` 数组(静默被链接器裁掉) |
| `"App" has not been registered` | metro 跑错工程(8081 被别的项目占着) |
| Android undefined symbol | superlib 被改成了 cdylib——必须 staticlib |
