# jsi-turbo 使用指南(本地路径集成)

(English version: [usage.md](usage.md))

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
- `react-native-rust-turbo` npm 包自带 jsi-rs 的 Rust 源码——
  **不需要 jsi-rs checkout**。(开发框架本身是例外:从 checkout
  用 `file:` 安装,构建钩子每次构建时从 checkout 同步 crates)

## 1. 接入你的 app(每 app 一次)

### 1.1 安装两个包

```bash
# 核心包:平台 carrier + 构建钩子 + 框架 crates
npm install react-native-rust-turbo

# 你的模块包(见第 2 节;开发期直接本地路径)
npm install ./native/my-math-utils
```

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
# 框架 crates 来自核心包的 rust/ 目录(node_modules 里的兄弟包)
# ——保证 cargo 图里只有一份
jsi = { path = "../react-native-rust-turbo/rust/jsi", default-features = false, features = ["macros", "serde"] }
jsi-turbo = { path = "../react-native-rust-turbo/rust/jsi-turbo" }
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
  → 生成聚合 superlib(<app>/node_modules/.cache/rust-turbo/;
    框架 crates 来自核心包的 rust/ 目录,dev 安装时从 jsi-rs checkout
    同步)
  → 一次 cargo 构建 → 单个 librust_app.a → 链入平台产物
JS import 生成 TS → bootstrap 触发 carrier → RustTurbo 代理安装
  → get(name) 惰性构造模块(plain object + prototype + NativeState)
```

## 6. 方法与类型规则(完整)

### 6.1 方法签名

```rust
#[RustTurboModule("MathUtils")]
impl MathUtils {
    // 普通方法:&self + 任意数量参数(可空)+ Result 返回
    pub fn add(&self, a: f64, b: f64) -> anyhow::Result<f64> { Ok(a + b) }

    // 需要 JSI 对象操作时,第二个参数收 RuntimeHandle(可选,按需)
    pub fn make_buffer<'rt>(
        &self, rt: &mut RuntimeHandle<'rt>, len: f64,
    ) -> anyhow::Result<JsiArrayBuffer<'rt>> { /* ... */ }

    // getter/setter:合并为 JS 属性,名字去掉 get_/set_ 前缀
    #[turbo(getter)] pub fn get_version(&self) -> anyhow::Result<String> { /* ... */ }
    #[turbo(setter)] pub fn set_cache_size(&self, size: f64) -> anyhow::Result<()> { /* ... */ }

    // async:方法体跑在 worker 池,JS 侧拿到 Promise
    pub async fn fetch(&self, url: String) -> anyhow::Result<String> { /* ... */ }
}
```

规则:

- 所有方法返回 `anyhow::Result<T>`,错误经 JS 异常抛出(带方法名)
- `rt: &mut RuntimeHandle` 只在需要 JSI 句柄(如 ArrayBuffer)时传,
  且方法需要显式生命周期 `<'rt>`
- `async fn` 的参数与返回值**不能**出现 JSI 类型(HRTB 约束,编译期
  拒绝);需要跨线程传句柄的场景用 `TurboCallback` / `TurboObject`
- 自定义构造:`#[RustTurboModule("X", ctor = "X::create")]`,任意
  `fn() -> anyhow::Result<X>` 路径,不要求 `Default`

### 6.2 参数/返回类型速查

| Rust 类型 | TS 类型 | 备注 |
|---|---|---|
| `f64` / `i32` / `u32` / `bool` | `number` / `boolean` | JS number 语义 |
| `String` | `string` | |
| `Vec<T>` / `Option<T>` | `T[]` / `T \| null` | 嵌套 serde |
| `#[derive(TurboStruct)]` 结构体 | `interface` | 深拷贝,见 7.1 |
| `JsiArrayBuffer<'rt>` | `ArrayBuffer` | 零拷贝,仅 sync 方法,见 7.3 |
| `TurboObject` 类型 | `interface` | 引用语义,见 7.4 |
| `Arc<T: TurboObject>` 参数 | 同上 | 从 JS 对象还原同一实例 |
| `TurboCallback` 参数 | `(arg: T) => void` | JS 函数,见 7.5 |

## 7. 进阶特性

以下特性全部在 `example/native/jsi-demo-module` 有可跑范例,
`jsi-tests/tests/` 有对应断言。

### 7.1 TurboStruct(值语义结构体)

```rust
#[derive(Serialize, Deserialize, TurboStruct, PartialEq, Debug)]
pub struct Point { pub x: f64, pub y: f64, pub label: String }
```

- 双向 serde 深拷贝:JS 收到普通对象,传回 Rust 反序列化
- 生成 `generated/rust-turbo/structs/Point.ts` 的 `interface`
- 适合小、不可变的数据;大对象/有身份的对象用 7.3/7.4

### 7.2 async → Promise

- 方法体在 jsi-turbo worker 池执行(不阻塞 JS 线程)
- resolve/reject 经 CallInvoker 跳回 JS 线程
- demo:`await turbo.slowAdd(20, 22)`

### 7.3 ArrayBuffer 零拷贝

```rust
// Rust → JS:Vec 所有权移交,JS ArrayBuffer 背后就是这块内存,
// GC 回收 buffer 时才 drop Rust 侧内存
pub fn make_buffer<'rt>(&self, rt: &mut RuntimeHandle<'rt>, len: f64)
    -> anyhow::Result<JsiArrayBuffer<'rt>>
{
    Ok(JsiArrayBuffer::from_vec(vec![0u8; len as usize], rt))
}

// JS → Rust:借用,同步调用期间原地读写,双向可见
pub fn checksum<'rt>(&self, rt: &mut RuntimeHandle<'rt>,
    buf: JsiArrayBuffer<'rt>) -> anyhow::Result<f64>
{
    Ok(buf.data(rt).iter().map(|&b| b as u64).sum::<u64>() as f64)
}
```

### 7.4 TurboObject(嵌套对象,引用语义)

模块方法返回非单例的 Rust 对象,JS 持有、调方法、再传回——
始终是同一实例:

```rust
pub struct Counter { value: AtomicU64 }

#[TurboObject]
impl Counter {
    pub fn increment(&self, by: f64) -> anyhow::Result<f64> { /* ... */ }
    #[turbo(getter)] pub fn get_value(&self) -> anyhow::Result<f64> { /* ... */ }
}

#[RustTurboModule("Canvas")]
impl Canvas {
    pub fn create_layer(&self, _rt: &mut RuntimeHandle, name: String)
        -> anyhow::Result<Layer> { /* ... */ }
    // Arc<T> 参数:从 JS 对象的 NativeState 还原同一 Arc
    pub fn count_points(&self, _rt: &mut RuntimeHandle, layer: Arc<Layer>)
        -> anyhow::Result<f64> { /* ... */ }
}
```

- 生命周期由 JS GC 决定(NativeState 析构 → Arc 归零)
- v1 取舍(设计记录见 `docs/nested-objects.md`):
  - 无 JS `===` 身份缓存(每次移交新建 JS 包装对象)
  - prototype 每次移交重建(高 churn 场景暂未优化)
  - 对象方法仅 sync(async 需要 dispatcher 传递,未做)

### 7.5 TurboCallback(JS 函数作参数)

```rust
#[derive(Default)]
pub struct Ticker { listeners: Mutex<Vec<TurboCallback>> }

#[RustTurboModule("Ticker")]
impl Ticker {
    pub fn on_tick(&self, cb: TurboCallback) -> anyhow::Result<()> {
        self.listeners.lock().unwrap().push(cb);
        Ok(())
    }
}

// 任意线程、任意时刻回推(serde 序列化参数,CallInvoker 跳回 JS 线程)
cb.call(TickEvent { index: 1.0, total: 3.0 });
cb.call_with_no_args();
```

- `TurboCallback` 是 `Send + Sync`,可存进模块长期持有
- 实测(RN 0.88.1,iOS/Android):回调里直接 `setState`,
  渲染即时发生(RuntimeScheduler 每任务后做 microtask checkpoint
  + updateRendering),**无需 `setTimeout` 包裹**

## 8. 生命周期与 reload

| 对象 | 作用域 | reload 后 |
|---|---|---|
| 模块注册表(linkme)/worker 池 | 进程级 | 存活 |
| `RustTurbo` 代理 + 模块实例 | per-runtime | 随旧 runtime 销毁,冷启动重装 |
| TurboObject 实例 | JS GC 决定 | 随 runtime 一起消失 |
| TurboCallback | 与捕获它的模块同寿 | 同上 |

- reload 是安全的默认形态:JS 侧所有状态清零重来
- Rust 侧若持有跨 runtime 的进程级资源(缓存、线程池),自行管理

## 9. 发布模块包

模块包本身就是普通 npm 包,发布后使用者只需:

```bash
npm install my-math-utils
cd ios && pod install
```

发布前检查:

- `package.json` 的 `files` 包含 `src/`、`generated/`、`Cargo.toml`
  (cargo 源码必须随包发布——聚合构建在**使用者的 app** 里跑,
  见第 5 节机制)
- `rust-turbo.modules` 列表与实际宏名逐一核对(iOS 锚,漏写静默消失)
- `crateName` 与 Cargo.toml 包名一致
- 版本号建议镜像 RN 版本(与 jsi-rs 版本策略对齐,0.88.x)
