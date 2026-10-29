# 设计讨论:jsi-rs 上 crates.io 的发布路径

状态:草案(P2)。结论见文末「建议路线」。平台模板(npm 侧)的
姊妹设计见 [`platform-templates.md`](platform-templates.md)。

## 问题

jsi-rs 目前只能以 git 仓库 + vendored submodule 的方式使用,无法 `cargo add`。
根因:**jsi-sys 的 build.rs 依赖 jsi.h 头文件**,而 crates.io 包不可能携带 RN
源码树。jsi.h 不是稳定 ABI——它必须与 app 实际链接的引擎(Hermes/JSC)版本
严格配对,所以「发布一个带预编译绑定的通用包」这条路天然走不通。

## 已落地的部分

- `JSI_DIR`(jsi-sys):指定含 `jsi/jsi.h` 的目录,默认 vendored RN 树
- `REACT_NATIVE_DIR`(jsi-rn):RN 包根目录,同上
- `bundled-jsi` feature:默认自己编译 `jsi.cpp`;RN app 关掉后符号由 RN 提供
  (单一 jsi 实现,无 ODR 风险)

也就是说「让 app 用自己的头」的机制已经存在,缺的是**默认值在 crates.io
安装场景下失效**(`vendor/react-native` 不存在),以及使用者的引导。

## 候选方案

### A. 必填环境变量(最小改动)

crates.io 安装后,build.rs 在 `JSI_DIR` 未设置且 vendored 树不存在时报出
可操作的错误(指路 node_modules 路径)。使用者在自己工程里配:

```bash
JSI_DIR=./node_modules/react-native/ReactCommon/jsi
REACT_NATIVE_DIR=./node_modules/react-native
cargo build
```

- ✅ 零机制新增,今天就能发
- ❌ 每个 Rust 构建入口(xcodebuild 的脚本、gradle 的 cargo 任务)都要注入
  环境变量,接线文档长

### B. 配置文件约定

jsi-sys 在 crate 根向上查找 `jsi-rs.toml` / 读 `package.json` 里的字段,
自动定位 `node_modules/react-native`。

- ✅ 配一次,所有构建入口共享
- ❌ 隐式行为排查困难;monorepo 下向上查找规则复杂

### C. 预生成绑定 + 无 build.rs

发布一个不编译 C++ 的版本:FFI 全走 `extern "C"` 声明,头文件只在类型
布局对得上时才安全——**jsi.h 的对象布局跨版本会变**(0.88 的 IRuntime
改造就是例子),此路会静默产生 UB,放弃。

### D. 发布为源码 + 版本镜像(当前版本策略的延伸)

crates.io 上每个 crate 版本 = RN 版本(0.88.1)。`jsi`/`jsi-macros`
(纯 Rust 部分)可以先发;`jsi-sys`/`jsi-rn` 依赖头文件路径,配合 A 的
必填环境变量发布。

## 建议路线(分两步)

1. **先发纯 Rust 层**:`jsi-macros`、以及未来不依赖 C++ 编译的部分,
   直接 `cargo publish`。这层没有头文件问题。
2. **`jsi-sys`/`jsi-rn` 走方案 A**:把「未找到 jsi.h」从隐式默认改为
   **清晰的构建错误 + 指路文档**(「你的 app 的 node_modules 里就有」),
   在 README 里给出 RN 工程的完整接线片段(xcodebuild 脚本与 gradle
   任务如何注入 `JSI_DIR`/`REACT_NATIVE_DIR`)。方案 B 作为后续增强,
   只在 A 的用户反馈证明接线太痛之后再做。

版本纪律:crates.io 版本号继续严格镜像 RN 版本;`cargo update` 升 jsi-sys
而不同步升 RN = 编译期头文件不匹配,应尽量在 build.rs 里加一个版本探测
(读 RN 的 package.json 版本比对)给出血色警告——列为后续任务。

## 后续任务(由本文派生)

- [ ] build.rs:`JSI_DIR` 未设置且 vendored 缺失时,输出指路错误(而非
      panic 于找不到头)
- [ ] README/crates.io 元数据:RN 工程接线文档(xcodebuild + gradle 注入
      环境变量)
- [ ] RN 版本探测比对(读 package.json,不匹配时 warning)
- [ ] `cargo publish` dry-run(`cargo package -p jsi-macros`)验证打包内容
