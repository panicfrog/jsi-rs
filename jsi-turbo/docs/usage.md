# jsi-turbo 使用指南(本地路径集成)

用 Rust 写 React Native TurboModule:一个属性宏就是全部 spec,框架
自动生成原生方法表、自动注册、TypeScript 绑定,并把 cargo 构建接进
平台构建——**没有任何手动 cargo/拷贝步骤**。本指南按「接入 app」→
「写模块」→「排错」的顺序覆盖本地路径(`file:`)集成;完整可跑的
范例就是本仓库的 `example/`。

```rust
#[RustTurboModule("MathUtils")]
impl MathUtils {
    pub fn add(&self, _rt: &mut RuntimeHandle, a: f64, b: f64) -> anyhow::Result<f64> { Ok(a + b) }
    pub async fn fetch(&self, url: String) -> anyhow::Result<String> { /* ... */ }
    #[turbo(getter)]
    pub fn get_version(&self) -> anyhow::Result<String> { /* ... */ }
}
```

```ts
// 编译期自动生成,JS 侧唯一入口
import { getMathUtils } from 'my-math-utils/generated/rust-turbo/MathUtils';
```

## 0. 前置条件

- Rust(rustup;`rustup target add aarch64-apple-ios-sim aarch64-apple-ios
  aarch64-linux-android`),Android 需 Android SDK/NDK + JDK 21,
  iOS 需 Xcode + CocoaPods
- jsi-rs 仓库一份 checkout(无需 `git submodule update`,见下)

## 1. 接入你的 app(每 app 一次)

### 1.1 安装两个包

```bash
# 核心包:平台 carrier + 构建钩子(必须)
npm install <jsi-rs 路径>/jsi-turbo/npm

# 你的模块包(见第 2 节;开发期直接本地路径)
npm install ./native/my-math-utils
```

核心包经 `file:` 安装为 symlink,build hook 会自动定位 jsi-rs
checkout(核心包在 `node_modules` 里时总能找到),**无需环境变量**。

### 1.2 iOS

```bash
cd ios && pod install     # autolinking 自动发现 RustTurbo pod
# Xcode 构建或 CI 命令照旧;首次编译前 pod 的 script phase 自动跑
# cargo 聚合构建(平台/配置自动映射 Rust target):
xcodebuild -workspace MyApp.xcworkspace -scheme MyApp ...
```

### 1.3 Android

```bash
cd android && ./gradlew assembleDebug
# rustTurboBuild task 在 CMake 前自动跑 cargo(build.js 自注入 NDK
# 工具链,无需 cargo-ndk);ABI 跟随 gradle.properties 的
# reactNativeArchitectures(默认 arm64-only)
```

完成。之后**改 Rust 代码只需正常构建**,平台钩子自动重编。

## 2. 写一个模块包

一个模块包 = 一个 Rust crate + npm 元数据:

```
my-math-utils/
  package.json     # 见下
  Cargo.toml       # crate-type = ["lib"]
  src/lib.rs       # #[RustTurboModule] impl 块
  generated/       # 宏生成的 TS(构建时自动更新,建议 .gitignore)
```

`package.json` 的关键字段:

```json
{
  "name": "my-math-utils",
  "version": "0.1.0",
  "main": "generated/rust-turbo/MathUtils.ts",
  "rust-turbo": {
    "crate": ".",
    "crateName": "my-math-utils",
    "modules": ["MathUtils"]
  }
}
```

- `crate`:crate 目录(相对包根;通常就是 `"."`)
- `crateName`:**必须与 Cargo.toml 的 `[package].name` 完全一致**
  (cargo 的 path 依赖按名字精确匹配,连字符/下划线不互认)
- `modules`:所有 `#[RustTurboModule("...")]` 的模块名列表(用于 iOS
  的 FORCE_LINK 锚,漏写则该模块在 iOS 上静默消失)

`Cargo.toml` 依赖:

```toml
[lib]
crate-type = ["lib"]

[dependencies]
jsi = { path = "<jsi-rs 路径>/jsi", default-features = false, features = ["macros", "serde"] }
jsi-turbo = { path = "<jsi-rs 路径>/jsi-turbo" }
linkme = "0.3"
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
```

方法规则速记:`&self` + 可选 `rt: &mut RuntimeHandle` 第二参 +
`anyhow::Result<T>`;类型支持 `f64/i32/u32/bool/String/Vec/Option`、
`#[derive(TurboStruct)]` 结构体、`JsiArrayBuffer`(零拷贝)、
`#[TurboObject]`(嵌套对象)、`TurboCallback`(JS 回调);`async fn`
暴露为 Promise(签名禁 JSI 类型)。完整规则见 `jsi-turbo/src/lib.rs`
的文档注释与 `example/native/jsi-demo-module`。

## 3. JS 侧使用

```ts
import { getMathUtils } from 'my-math-utils/generated/rust-turbo/MathUtils';

const math = getMathUtils();        // 首次调用才构造(惰性单例)
math.add(19, 23);                   // 42
await math.fetch('...');            // Promise
math.version;                       // getter
```

诊断:`RustTurbo.names()` / `.has(name)` / `.version` / `.buildType`
(JS console 可直接查注册状态)。

## 4. 常见报错 → 原因

| 报错/症状 | 原因 |
|---|---|
| `RustTurbo bindings failed to install` 且 `get('RustTurbo')` 为 null | 核心包没进 autolinking:多半是 package.json 里 `file:` 路径层级错(用 `npm ls` 查——出现 `invalid` 即路径错,RN CLI 会**静默**丢弃该包) |
| pod install 后 Podfile.lock 里没有 RustTurbo | 同上;`npx @react-native-community/cli config` 看依赖是否被发现 |
| `no matching package named ... found`(cargo) | 模块包元数据 `crateName` 与 Cargo.toml 包名不一致 |
| `RustTurbo.get('X')` 报未注册 | `modules` 列表漏写该模块(iOS FORCE_LINK 没锚,被链接器裁掉) |
| 白屏 / `"App" has not been registered` | metro 没跑或跑错工程(8081 端口同一时间只能服务一个 app) |
| Android undefined symbol | superlib 被改成了 cdylib——聚合库必须是 staticlib |
| `could not find jsi/jsi.h` / React Native 目录 | 核心包生成的聚合 crate 自带 `.cargo/config.toml` 指向 app 的 node_modules;若手动跑 cargo 报此错,说明不在平台钩子的环境里 |

## 5. 工作机制(一图流)

```
npm install(核心包 + 模块包)
pod install / gradle 配置(autolinking 发现 RustTurbo)
平台构建 → 构建钩子(pod script phase / rustTurboBuild task)
  → build.js 扫 node_modules 里所有 rust-turbo 模块包
  → 生成聚合 superlib(<app>/node_modules/.cache/rust-turbo/)
  → 一次 cargo 构建 → 单个 librust_app.a → 链入平台产物
JS import 生成 TS → bootstrap 触发 carrier → RustTurbo 代理安装
  → get(name) 惰性构造模块(plain object + prototype + NativeState)
```
