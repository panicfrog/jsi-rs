# 设计:可发布的平台模板(P2)——零胶水接入

状态:草案。参考 Nitro(v0.37.1)的构建集成方式调研结论,
2026-10-29 记录。

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
2. **多模块聚合** —— linkme 分布式切片要求所有模块的 `.o` 进同一个
   静态库,而 RN autolinking 是按 npm 包逐个链库的。两个方向:
   - 单聚合 crate(现状 superlib 模式):npm 包只是壳,依赖一个
     用户自建的聚合 crate;
   - 或允许多个 `.a` 各自带 carrier(TurboModule 名字不同即可共存),
     每包自包含。后者对 autolinking 更友好,代价是每包一个
     TurboModule/代理入口(`RustTurbo` 代理改为可重入/可共存)。
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
4. 多包共存问题(机制问题 2)做 spike 后定方向;
5. 与 crates.io 路线的指路错误联动(build.rs 报错指向 npm 包文档)。
