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

**怎么做**(设计见 [`docs/platform-templates.md`](docs/platform-templates.md),
基于对 Nitro v0.37.1 构建集成的调研):仿 Nitro 的
`+autolinking.{rb,gradle,cmake}` 模式——宏/构建脚本生成 carrier
三件套和构建描述文件,npm 包携带 podspec + gradle 走 RN 标准
autolinking;cargo 由生成的构建脚本触发(iOS `script_phase`,
Android gradle task),`JSI_DIR`/`REACT_NATIVE_DIR` 自动指向
`node_modules`。关键待决:多模块 `.a` 聚合 vs 多包共存。

**验收**:新 app `npm install` 即用,不拷贝任何文件、不改
Podfile/CMakeLists。

### P2 · crates.io 发布路径(大)

**背景**:jsi-turbo 依赖本仓库(git submodule 的 vendored RN 头),
使用者无法从 crates.io 安装。

**进展**:头文件注入机制已落地(`JSI_DIR` / `REACT_NATIVE_DIR` 环境变量,
`bundled-jsi` feature 关闭后由 app 提供 jsi 符号)。设计讨论见
[`docs/crates-io-publishing.md`](docs/crates-io-publishing.md),
结论:纯 Rust 层先发,jsi-sys/jsi-rn 走「必填环境变量 + 指路错误」,
派生任务列表在该文档末尾。

### P3 · 自定义构造器 ✅(已完成)

`#[RustTurboModule("X", ctor = "X::create")]`——任意 `fn() ->
anyhow::Result<X>` 路径,不再要求 `Default`。

### P3 · 注册重名的 debug 断言 ✅(已完成)

`ModuleDescriptor` 带 `rust_type`;`registry::lookup` / `register` /
`install` 在 debug 构建下对同名冲突断言并列出冲突的 Rust 类型;
`registry::duplicate_names()` 供宿主启动时主动检查。

### P3 · 嵌套对象(对象图) ✅(已完成)

`#[TurboObject]`:非单例 Rust 对象,模块方法返回 → JS 持有
(NativeState,GC 决定生命周期)→ 以 `Arc<T>` 参数传回(同一实例,
引用语义)。与模块共用 prototype/NativeState/external_size/__type
机制。设计取舍(v1:无 JS `===` 身份缓存、prototype 按次构建、
对象方法仅 sync)见 [`docs/nested-objects.md`](docs/nested-objects.md)。

### P3 · ArrayBuffer 零拷贝 ✅(已完成)

大数据双向传递不走 serde:
- **Rust → JS**:`JsiArrayBuffer::from_vec(vec, rt)` 把 Vec 所有权移交
  runtime——外部 ArrayBuffer(jsi.h 的 `createArrayBuffer(shared_ptr<
  MutableBuffer>)`,Hermes 经 `setExternalDataBlock` 保留共享指针),
  GC 回收 JS buffer 时才 drop Rust 内存。真·零拷贝。
- **JS → Rust**:方法参数收 `JsiArrayBuffer<'rt>`(借用,同步调用
  保证方法返回前有效),`data(rt)` 原地读写,双向可见。
- 宏:JsiArrayBuffer 归入 JSI handle 类型(仅 sync 方法),TS 映射
  `ArrayBuffer`;方法需显式生命周期 `fn f<'rt>(&self, rt: &mut
  RuntimeHandle<'rt>, buf: JsiArrayBuffer<'rt>)`。

### P3 · JS 回调参数 ✅(已完成)

方法参数收 JS 函数并异步回推(事件场景)。`TurboCallback`:宏在
invoker 里连同 `Dispatcher` 一起捕获,`Send + Sync` 可存进模块;
`call(arg)`(serde 序列化)/ `call_with_no_args()`,CallInvoker
路径跳回 JS 线程。已知事项:
- ~~**RN 集成注意**:JSI 回调里调 `setState` 渲染会被推迟,需
  `setTimeout` 包裹~~ **已证伪**(2026-10-29,iOS/Android 双端实测
  RN 0.88.1):回调里的 `setState` 即时渲染——同步 tick ~1ms,
  异步 tick(CallInvoker 跳回)每秒逐个刷屏,无需任何包装。原因:
  RN 的 RuntimeScheduler 在每次 JS 任务后做 microtask checkpoint
  并 updateRendering(源码级确认)。demo 已去掉 `setTimeout`。
- **在飞回调 vs reload**(未证实的猜测,暂不处理):理论上 worker
  线程上排队的回调可能在 runtime 销毁后 dispatch。实测(2026-10-29,
  Android,12 次快速 reload、约 30 个在飞回调)未出现任何崩溃;
  Nitro 也未做任何处理。除非将来出现真实崩溃报告,否则不投入。

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
