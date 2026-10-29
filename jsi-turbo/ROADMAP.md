# jsi-turbo 路线图与协作指南

用 Rust 写 React Native TurboModule 的框架,思路来自
[Nitro Modules](https://github.com/margelo/nitro),但 **Rust 宏是唯一的
spec 源**:一个属性宏生成原生方法表、自动注册和 TypeScript 绑定,不
写任何平台胶水。

```rust
#[RustTurboModule("TurboDemo")]
impl TurboDemo {
    pub fn add(&self, _rt: &mut RuntimeHandle, a: f64, b: f64) -> anyhow::Result<f64> { Ok(a + b) }
    pub async fn fetch(&self, url: String) -> anyhow::Result<String> { /* ... */ }
    #[turbo(getter)]
    pub fn get_version(&self) -> anyhow::Result<String> { /* ... */ }
}
```

```ts
// 编译期自动生成,JS 侧唯一入口
import { getTurboDemo } from '<模块crate>/generated/rust-turbo/TurboDemo';
```

**上手材料**:
- 完整可跑的例子:`example/`(含 iOS/Android 双端验证过的接线)
- 平台接入一次性步骤:`jsi-turbo/carrier/README.md`
- API 文档:`jsi-turbo/src/lib.rs` 及各模块 doc 注释

---

## 当前状态(更新于 2026-10-29,分支 `feature/jsi-turbo`)

Phase 0–3 已完成并双端验证:

| 阶段 | 内容 | 验证 |
|---|---|---|
| 0 | 技术验证 spike:linkme 在 iOS staticlib 存活条件(需 force 锚);NativeState 绑定形态 | `native_state_spike.rs` 4/4 |
| 1 | 垂直切片:宏 → 方法表 → linkme 注册 → 惰性 `RustTurbo.get(name)` → prototype 方法分发 → TS 生成 | `turbo_basic.rs` |
| 2 | async fn → Promise(线程池 + CallInvoker 跳回 JS 线程)、`#[derive(TurboStruct)]`、getter/setter 合并 | `turbo_async.rs`、`turbo_struct.rs` |
| 3 | 平台 carrier(每 app 一次)+ superlib 聚合库 + example 改造 | iOS/Android 模拟器端到端 |

验证基线:`cargo test -p jsi-tests` 30/30;example 的 `tsc --noEmit`、
jest;双端模拟器运行截图核对(方法/getter/结构体/async 全对)。

## 架构速览(30 秒)

```
编译期   #[RustTurboModule] → 方法表(const)+ linkme 条目 + force 锚 + generated/rust-turbo/*.ts
进程级   TURBO_MODULES 链接器自动收集(无显式注册代码)+ 手动 register() 通道 + worker 线程池
每 runtime  carrier → rust_turbo_install → globalThis.RustTurbo = { get(name) }(仅此一个代理)
惰性构造   RustTurbo.get('X') → 查表 → Arc<X> → prototype(defineProperty 预装方法)
          → Object.create(prototype) → setNativeState(Arc<X>) → 缓存单例
调用     方法 = prototype 普通属性 → this.getNativeState() 取回 Arc<X> → 调真实方法
回收     JS 对象 GC → NativeState 析构 → Arc 归零 → Rust 对象释放
```

关键源码:`jsi-turbo-macros/src/{parse,emit}.rs`(宏)、
`jsi-turbo/src/{registry,proxy,promise}.rs`(运行时)、
`jsi-sys/src/ffi/native_state.rs`(NativeState 绑定)。

## 如何验证你的改动

```bash
cargo test -p jsi-tests                 # 真 Hermes 测试桩
cd example && npx tsc --noEmit          # 生成的 TS 类型
# 双端构建 + 模拟器运行:见 example/README.md(构建命令、模拟器 UDID 等)
```

## 待办任务(可认领)

按优先级排列;认领后照「做什么 / 验收」干即可,不确定的先开 issue 讨论。

### P1 · 诊断 API ✅(已完成)

**背景**:模块注册信息只在 Rust 侧,JS 无法主动查询。新人最常见的坑
(iOS 忘加 FORCE_LINK 导致模块静默消失)只能靠报错发现。

**做什么**:给代理(`jsi-turbo/src/proxy.rs` 的 `get` 同级)加 host fn:
`names()` / `has(name)` / `version`;debug 构建给模块实例加 `__type`
属性(参照 Nitro 的 `HybridNitroModulesProxy`)。

**验收**:`turbo_basic.rs` 增加断言;example 的 debug 构建里
`RustTurbo.names()` 返回 `['TurboDemo']`。

### P1 · `setExternalMemoryPressure` 钩子 ✅(已完成)

**背景**:Hermes GC 看不见 NativeState 背后的原生内存,模块持有大
缓冲时回收滞后。底层绑定已就绪
(`JsiObject::set_external_memory_pressure`,见 `jsi/src/native_state.rs`)。

**做什么**:`TurboModuleObject` trait 加 `fn turbo_external_size(&self)
-> usize { 0 }`(默认实现);proxy 构造实例时自动调用上报。

**验收**:trait 方法有测试;持有大 Vec 的测试模块在
`turbo_external_size` 返回非零时行为不变。

### P2 · 可发布的平台模板

**背景**:carrier 目前是「拷贝文件」模式(每 app 一次),对使用者仍
有接线成本。

**做什么**:把 carrier 做成可发布的 pod / Android library(AAR 或
maven 模块),superlib 参数化(podspec 用占位符指向用户产物);参考
`example/native/rust-app` 的 bridge 形态。

**验收**:新 app 不拷贝任何文件,只依赖发布的库 + 配置产物路径。

### P2 · crates.io 发布路径(大)

**背景**:jsi-turbo 依赖本仓库(git submodule 的 vendored RN 头),
使用者无法从 crates.io 安装。

**做什么**:评估解耦方案——jsi-sys 允许使用 app 自带 RN 头
(build.rs 环境变量注入 include 路径)或预生成 FFI 绑定。涉及
`jsi-sys/build.rs` 与版本兼容策略,建议先写设计讨论再动手。

### P3 · 自定义构造器

`#[RustTurboModule("X", ctor = "X::create")]`——宏目前只支持
`Default`。改 `jsi-turbo-macros/src/{parse,emit}.rs`。

### P3 · 注册重名的 debug 断言

同名模块重复注册(两个 crate 撞名)目前静默覆盖。在
`registry::lookup` / `registered_names` 加 debug_assert + 报错列出
冲突来源(Nitro 的做法)。

### P3 · 嵌套对象(对象图)

方法返回另一个 Rust 对象(非 TurboModule 单例),TS 类型递归生成。
需要对象身份(WeakObject 缓存,参照 Nitro 的 `_objectCache`)与 GC
协议设计,建议先出设计文档。

### P3 · ArrayBuffer 零拷贝

大数据双向传递不走 serde。`JsiArrayBuffer::data()` 已有,缺的是宏
类型映射(`&[u8]` / 自定义 buffer 类型)+ 生命周期看护。

### P3 · JS 回调参数

方法参数收 JS 函数并异步回推(事件场景)。依赖 CallInvoker
(`Dispatcher` 已就绪),需要 `JsiFn` 参数的转换与线程安全封装
(参照 Nitro 的 `AsyncJSCallback`)。

## 已知约束与坑(设计决策记录)

| 事项 | 结论 |
|---|---|
| iOS staticlib 丢模块 | rustc 按符号可达性裁剪归档;每模块宏生成 `#[no_mangle]` force 锚,superlib 的 `#[used] FORCE_LINK` 数组必须引用(spike 验证过 default/dead_strip/iOS sim) |
| Android 用 .so 还是 .a | **必须 .a(staticlib)**:cdylib 链接会丢弃 cxx 桥的 C++ 对象(as-needed)→ undefined symbol |
| 模块 crate 依赖 linkme | 必须直接依赖(宏生成代码硬编码 `::linkme` 路径,re-export 无效) |
| async 签名禁 JSI 类型 | 宏与 `promise::spawn` 的 HRTB(`for<'a> IntoValue<'a>`)约束双重保证;`&mut` 不变性是各处 reborrow 的原因 |
| 头文件 inline | jsi-sys 三个头的 87 个函数已 inline 化;新增头文件函数必须 inline(多归档成员包含同头会重复符号) |
| Android ABI | 由 `gradle.properties` 的 `reactNativeArchitectures` 控制;本项目约定 arm64-only |
| reload 语义 | 注册表/线程池进程级;代理与实例 per-runtime,冷启动自动重装 |

## 分支与提交

- 开发在 feature 分支(从 main 切出),见根目录 CLAUDE.md
- 提交信息不带 AI 署名尾行(CLAUDE.md 约定)
