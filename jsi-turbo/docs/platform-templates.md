# 设计:可发布的平台模板(P2)——零胶水接入

状态:方案定案(2026-10-29 spike 后更新,见文末「共存性 spike 结论」)。
参考 Nitro(v0.37.1)的构建集成方式调研结论。

## 问题

现状的接入成本集中在 carrier(`carrier/README.md` 的手工步骤):
拷贝 `.mm`/`.kt`/`.cpp`、改 Podfile/CMakeLists/MainApplication、
手动跑 cargo 拷 `.a`。「每 app 一次」听起来轻,但每一步都可能抄错,
且 cargo 产物路径与平台构建是脱节的(改了 Rust 要手动重拷)。

## Nitro 的做法(调研结论)

Nitro 的「零代码」实际是三层分工:

1. **codegen 产物随 npm 包发布**(库作者手动 `npx nitrogen`,
   生成物提交进包,消费者永不跑 codegen);
2. **生成的构建描述文件**:`+autolinking.rb`(podspec 用)、
   `+autolinking.gradle`、`+autolinking.cmake`——它们是给构建系统
   读的「数据文件」,被模板自带的**三行一次性 include** 挂上;
3. **App 侧 100% 走 RN 标准 autolinking**(npm 包里有 podspec +
   build.gradle,`use_native_modules!` / gradle plugin 自动发现),
   连对核心库 NitroModules 的依赖都由 `+autolinking.rb` 传递声明。

没有任何 pod-install / gradle-sync 钩子;构建期不跑 codegen。

## 我们的对应方案

我们的优势:**spec 源就是 Rust impl 块**,宏在 `cargo build` 时
已经拥有一切信息(模块名、方法表、force 锚),且没有 Swift/Kotlin
桥接层——生成物比 Nitro 少一大半。

目标形态:一个 Rust 模块 crate 发布为 **npm 包**,包含:

```
<module>-turbo/
  package.json          # RN autolinking 元数据(react-native.config.js 或字段)
  <Module>.podspec      # 三行式:load 生成的 autolinking.rb
  android/build.gradle  # apply from 生成的 autolinking.gradle
  android/CMakeLists.txt# include 生成的 autolinking.cmake
  generated/            # 构建时由宏/构建脚本产出(或随包发布)
    rust-turbo/*.ts         # TS 绑定(现有)
    ios/+autolinking.rb     # 源文件/xcconfig/依赖声明
    android/+autolinking.{gradle,cmake}
    carrier/                # RustTurboModule.mm/.kt/Bindings.cpp(现有 carrier 内容)
  build.rs 或 bin      # 触发 cargo 构建 → .a → 拷到平台构建可见的位置
```

关键机制问题(按优先级):

1. **谁跑 cargo** —— Nitro 不需要编译原生代码之外的东西,而我们必须
   编 Rust。候选:
   - a. podspec 的 `script_phase` / `prepare_command` 调 `cargo build`
     (iOS);gradle task 调 `cargo ndk`(Android)——构建系统集成最深,
     改 Rust 即自动重编,但要求构建机有 Rust 工具链;
   - b. CI 预编译产物随包发布(免工具链,但版本矩阵爆炸,且违背
     「jsi.h 必须与 app 的引擎配对」——静态库可以,jsi 符号本来就
     由 app 侧提供,见 `bundled-jsi` 关闭模式)。
   - **已定(2026-10-29):走 a**,cargo 自己的构建命令由平台构建
     钩子触发——iOS podspec `script_phase` / Android gradle task,
     带上正确的 target 与 `JSI_DIR`/`REACT_NATIVE_DIR` 注入。
   - 补充:pnpm v12.4+ 的 cargo 依赖支持(`pnpm add crate:serde`,
     multi-ecosystem 实验特性)可让用户用一个 `pnpm install` 同时管
     npm + Cargo 两张依赖图(crate 源码进 pnpm store,`cargo build`
     离线编译)。它只做依赖分发、不编译,不改变上面的触发点结论,
     仅作为可选增强写进用户文档,不是依赖项。
2. **模块注册与「保证被链入」** —— 与 Nitro 的本质区别:Nitro 的
   注册表是运行时 map,必须有人执行填充代码(Android `JNI_OnLoad` →
   `registerAllNatives()`,iOS ObjC `+load`);我们的注册是**链接期
   静态数据**(linkme 切片),`.so`/二进制加载即就位,不存在需要被
   调用的注册函数。Android 链路:`SoLoader.loadLibrary`(RN 模板
   自带)→ linkme 切片随 .so 就位 → JS import 触发 carrier →
   `rust_turbo_install` 装代理 → `get(name)` 遍历切片。carrier 里的
   JNI 函数只是 fbjni 方法绑定,不是注册。
   真正的问题是**链接器可能丢弃无引用的 `.o`**:
   - Android:每包的 `.a` 链进**本包自己的** `.so`(autolinking
     天然按包建库),superlib 的 `#[used]` 锚数组在包内自引用即可;
   - iOS:`#[used]` 只保证符号进二进制,dead_strip 仍可能裁——
     真正保命的是「有代码引用锚」(已踩过的坑)。npm 包形态下由
     **生成的 carrier `.mm` 引用本包锚数组**承担(它必须被编译
     进 app),每包自带,互不依赖。
   由此多包聚合问题(原机制问题)的结论:**每包自包含可行**——
   每包一个 `.a` + 一份锚引用 + 一个 TurboModule 入口,天然可共存;
   剩余待 spike 的只有代理共存(`RustTurbo` 代理支持多次 install:
   名字空间合并,或按包前缀/独立代理)。
3. **环境变量注入**(`JSI_DIR`/`REACT_NATIVE_DIR`,见
   [crates-io-publishing.md](crates-io-publishing.md))——在生成的
   gradle/cargo 脚本里自动指向 `node_modules/react-native`,
   用户无需手配。

## 与 crates.io 发布的关系

两条发布线互补、不冲突:

- **crates.io**:Rust 侧依赖(`jsi`、`jsi-macros` 先发;
  `jsi-sys`/`jsi-rn` 走必填环境变量);
- **npm**:平台模板 + TS 绑定 + 构建脚本,把「每 app 一次」的
  carrier 变成 autolinking 自动发现。

npm 包名建议 `create-rust-turbo` / `<module>-turbo` 约定。

## 分步实施(草案)

1. 把 carrier 三件套改为模板参数化(模块名/产物路径占位),
   手工验证与现状等价;
2. 写 `jsi-turbo-gen`(或宏的 postrun):从注册表读全部模块,
   生成 `+autolinking.*` 与 carrier 文件;
3. example 改造成 npm 包形态,验证「npm install 即用、零拷贝」;
4. 代理共存 spike:两个 npm 包(各自 `.a` + carrier)装进同一 app,
   验证 linkme 切片按包各自可见、`RustTurbo` 代理多次 install 的
   名字空间合并;
5. 与 crates.io 路线的指路错误联动(build.rs 报错指向 npm 包文档)。

## 共存性 spike 结论(2026-10-29)

两个独立自包含 `.a`(各自依赖完整 jsi-turbo)链进同一 iOS 二进制:

- **无 duplicate symbol**:静态归档按需拉入天然去重——符号先解析者
  入表,后到归档的重复定义成员不再被拉入,运行时只链一份;
- **linkme section 合并**:两包的模块条目都进最终 slice,两个 force
  锚都被链入,模块正常注册;
- **约束**:同 app 内所有包必须依赖同一 jsi-turbo 版本(运行时只保
  留一份;聚合构建下由 cargo 统一解析天然保证)。

## 形态定案:分发自包含,构建聚合

由此采用 Nitro 的核心库模式(而非每包预构建 `.a`):

- **`react-native-rust-turbo` 核心 npm 包(每 app 一份)**:carrier +
  单个 `RustTurbo` pod/TurboModule;构建钩子(pod `script_phase` /
  Android gradle task)扫描 `node_modules` 中带 rust-turbo 标记的
  模块包 → 生成聚合 superlib(依赖 + FORCE_LINK)→ 一次 cargo 构建
  出单 `.a`(自动注入 `JSI_DIR`/`REACT_NATIVE_DIR`,见
  [usage.md](usage.md));
- **模块 npm 包(N 份,纯源码)**:Rust crate 源码 + 宏生成的 TS +
  `package.json` 元数据(crate 路径);不含平台文件、不含预构建产物;
- 用户侧:`npm install react-native-rust-turbo <module>` 即用,走 RN
  标准 autolinking,零拷贝、零 Podfile/CMake 修改。

单次 cargo 构建的额外收益:依赖图统一解析(版本一致性)、增量构建
一次跑完、无跨归档符号解析的隐式依赖。
