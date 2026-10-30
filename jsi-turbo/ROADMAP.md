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
- **使用指南(接入 app、写模块、类型规则、排错)**:
  [`docs/usage.md`](docs/usage.md)
- 完整可跑的例子:`example/`(含 iOS/Android 双端验证过的接线)
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

### P2 · 可发布的平台模板(进行中)

**背景**:carrier 目前是「拷贝文件」模式(每 app 一次),对使用者仍
有接线成本。

**怎么做**(设计与共存性 spike 结论见
[`docs/platform-templates.md`](docs/platform-templates.md)):Nitro 的
核心库模式——`react-native-rust-turbo` 核心 npm 包持有 carrier 和
构建钩子,扫描 node_modules 中的 rust-turbo 模块包,生成聚合
superlib,单次 cargo 构建。

**进展**:
- ✅ 共存性 spike:多份自包含 `.a` 无符号冲突(归档按需拉入天然去
  重),linkme 条目跨归档合并
- ✅ 构建钩子核心(`npm/scripts/build.js`):发现 + 聚合生成 + 单次
  构建(含 symlink node_modules 支持)
- ✅ **iOS 零胶水集成**:核心 npm 包(podspec + `script_phase` 自动
  cargo + `prepare_command` placeholder),纯 autolinking 端到端验
  证(npm install → pod install → xcodebuild → 模拟器)。踩坑记录:
  CocoaPods 在 pod install 时对 vendored `.a` 做**存在性 + lipo 架构**
  双重检查,空 archive 会被静默丢弃 → 用预置 504 字节 arm64
  placeholder(`scripts/placeholder.a`)
- ✅ **Android 零胶水集成**(`21b54d6`):核心包 Android library 模块
  (autolinking 自动注册,零 Java/CMake 改动)——`rustTurboBuild`
  gradle task 自动 cargo(build.js 自注入 NDK 工具链,免 cargo-ndk),
  CMake 编 JNI glue 进 `librustturbo.so` 并静态链入聚合 `.a`
  (`buildFeatures.prefab` 解 ReactAndroid/fbjni;RN 0.88 头要 C++20)。
  assembleDebug → 模拟器全绿(含 async Promise)。踩坑记录:**生成
  产物绝不能放 npm 包目录内**——`file:` 本地包是 symlink,RN
  autolinking CLI 扫依赖树时撞上 cargo `target/` 直接 OOM(4GB 堆),
  产物移到 `<app>/node_modules/.cache/rust-turbo`
- ✅ **example 迁移到 npm 包形态**(`afa74fd`):jsi-demo-module 成为
  模块包,手工 superlib/carrier/Podfile/MainApplication 接线全部移除,
  双端零胶水(autolinking + 构建钩子自动 cargo),冷启动/Fast
  Refresh/全量 reload 三种场景验证。踩坑记录:① package.json 的
  `file:` 路径层级算错会让 npm 依赖图 invalid,RN CLI **静默**把包从
  autolinking 里丢掉——症状是 RustTurbo pod 消失、`get('RustTurbo')`
  返回 null、冷启动即"bindings failed to install";排查手段是给
  `installJSIBindingsWithRuntime` 插桩看它是否被调 + `npm ls` 查
  invalid。② cargo path 依赖按包名精确匹配(连字符/下划线不互认),
  元数据里的 crateName 必须是真实包名,代码引用再转标识符
- ✅ react-android prefab 依赖随 app 的 RN 版本联动:build.gradle 从
  app 的 `node_modules/react-native/package.json` 动态读版本(不再
  硬编码——patch 版本天然兼容,只有 major.minor 偏离才 warning);
  Android 模拟器端到端验证(构建 → 解析到 app 实际版本 → 装机运行)。
  fbjni 不随 RN 版本联动(独立演进),保持默认值 + `ext.fbjniVersion`
  覆盖开关
- ✅ 版本一致性校验(build.js `checkVersions`):app 的 RN 版本 vs
  jsi-rs 目标版本(读 `jsi/Cargo.toml`,crate 版本即 RN 版本),
  major.minor 偏离或 RN 缺失时 warning 不 fail;覆盖 iOS/Android
  两条构建路径(cargo 侧比 gradle 配置侧更贴近 jsi.h 编译现场)。
  三种场景单测验证(匹配静默/偏离告警/缺失告警)+ rustTurboBuild
  实跑通过
- ✅ npm README(`jsi-turbo/npm/README.md`:定位、安装、机制、排错表、
  指回仓库 usage 文档)。踩坑记录:`files` 里显式列出的路径**压过**
  `.npmignore`(npm 规则),`"android/"` 整目录列入会把 gradle 构建产物
  一起打进包(npm pack 实测 832MB/223 文件)——改为逐文件白名单后
  33kB/14 文件;`android/src/main/rust/`(rustTurboBuild 拷入的 `.a`)
  进根 .gitignore
- ✅ **包内自举(方案 a,2026-10-30)**:六个 crate 源码随 npm 包发布
  (prepack 拷入 `rust/`,兄弟布局保持 `../兄弟` path 依赖;fbjni 头
  文件一并随包——react-native 的 npm 包不含它)。dev 的 `file:` 安装
  无 prepack,build.js 每次构建从 checkout 同步 `rust/`。**crate 来源
  统一走 `node_modules/react-native-rust-turbo/rust/`**(模块包与
  superlib 同串引用;cargo 不做路径规范化,混用 symlink 真实路径会
  出双份 jsi-sys 撞 `links = "jsi"`)。tarball 安装双平台端到端验证
  (npm pack → 安装 → Android/iOS 全链 → 模拟器数值核对),dev 回退
  后同样验证。附带发现:生成的 `.cargo/config.toml` 从未生效过
  (cargo 只从工作目录向上找 config)——JSI_DIR/REACT_NATIVE_DIR 改为
  build.js 直接注入 cargo 进程环境
- ✅ **init 脚手架**(2026-10-30):`npx react-native-rust-turbo init
  <name>`(package.json 的 `bin`)。模板三名字由一个参数派生(npm 名
  即 crate 名、PascalCase 即模块名),产物含可运行示例模块。纯文件
  生成、registry 安装即可用,无需 clone 仓库。Android 端到端验证
  (init → 装进 example → 全链构建 → add=42/version=0.1.0 上屏)。
  开发中发现并修掉模板两个 bug:元数据键名 `rust_turbo` 应为
  `rust-turbo`;struct 需 `#[derive(Default)]`

**验收**:新 app `npm install` 即用,不拷贝任何文件、不改
Podfile/CMakeLists。

### P2 · 依赖独立(原「crates.io 发布」)✅ 核心已验证

**结论(2026-10-29)**:分发渠道不重要——crates.io 与本地 path 依赖
等价,**本质诉求是代码独立**:使用者的工程脱离本仓库的 vendored
submodule 也能用。已通过独立工程 spike 完整验证:

- 仓库外 cli init 的原生 RN app(`npx @react-native-community/cli init`,
  RN 0.88.1)+ 独立 cargo crate(superlib + 模块同 crate,path 依赖
  jsi-rs 各 crate,`default-features = false` 关 `bundled-jsi`)
- **`.cargo/config.toml` 的 `[env]` 相对路径**注入
  `JSI_DIR`/`REACT_NATIVE_DIR` 指向 app 自己的
  `node_modules/react-native`——所有 cargo 入口自动生效,零重复配置
- podspec + carrier 三件套照抄 `jsi-turbo/carrier/`
- iOS 模拟器端到端:getter/方法/async Promise(CallInvoker)/
  `RustTurbo.names()` 全部通过

**剩余小尾巴**(体验完善,非阻塞):
- ~~build.rs 指路错误~~ ✅ `JSI_DIR`/`REACT_NATIVE_DIR`/`FBJNI_DIR`
  缺失或指错时给出可操作的报错(含 `.cargo/config.toml` 示例)
- ~~README 补「本地依赖接线」章节~~ ✅ 见
  [`docs/usage.md`](docs/usage.md)(完整步骤
  + 常见报错表,从 spike 工程提炼)
- ~~cargo package dry-run~~(随 crates.io 降级砍掉;设计文档
  [`docs/crates-io-publishing.md`](docs/crates-io-publishing.md) 保留
  供未来需要时参考)

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
| async 执行器(2026-10-30 重做) | worker 池 cores−1(`available_parallelism`,保底 1)+ `futures_lite::future::block_on`(真 waker,挂起即 park)。依赖面只有 `futures-lite`;timer/IO 用户自加 `async-io`/`smol`(全局 reactor 与执行器无关,互通);tokio 生态库需模块自持 Runtime。**教训**:共享 `async-executor` + tick 循环宿主机正常、Android 真机上任务永不派发(worker 全 park、promise 挂死,根因未明),二分定位后弃用;`CatchUnwind` 保证 panic → reject 而非挂死 |
| 头文件 inline | jsi-sys 三个头的 87 个函数已 inline 化;新增头文件函数必须 inline(多归档成员包含同头会重复符号) |
| Android ABI | 由 `gradle.properties` 的 `reactNativeArchitectures` 控制;本项目约定 arm64-only |
| reload 语义 | 注册表/线程池进程级;代理与实例 per-runtime,冷启动自动重装 |

## 分支与提交

- 开发在 feature 分支(从 main 切出),见根目录 CLAUDE.md
- 提交信息不带 AI 署名尾行(CLAUDE.md 约定)
