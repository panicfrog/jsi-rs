# 设计:嵌套对象(对象图)—— `#[TurboObject]`

状态:✅ 已实施(v1)。本文是 ROADMAP「嵌套对象」条目的设计记录。

## 问题

TurboModule 是全局单例(`RustTurbo.get(name)`),但真实模块经常要返回
**非单例的 Rust 对象**:`Canvas.createLayer()` 返回一个 `Layer`,
JS 持有它、调它的方法、再传回 Rust。现状只有两条路:TurboStruct
(深拷贝、无身份、无行为)或把一切塞进单例——都不对。

## 方案:复用模块的包装机制

模块的 JS 侧包装已经是「NativeState 存 Rust 实例 + prototype 挂方法」
(proxy.rs)。对象用同一套,只是**入口不同**:

| | TurboModule | TurboObject |
|---|---|---|
| 入口 | `RustTurbo.get(name)` 全局注册表 | Rust 方法返回值 / JS 传参 |
| 实例数 | per-runtime 单例 | 任意多个 |
| 生命周期 | 与 runtime 同寿 | JS GC 决定(NativeState) |
| TS | `get{name}(): {iface}` | interface,作为类型出现在签名里 |

```
#[TurboObject]
impl Layer {
    pub fn add_point(&self, x: f64, y: f64) -> Result<()>
    #[turbo(getter)] pub fn get_opacity(&self) -> Result<f64>
}

#[RustTurboModule("Canvas")]
impl Canvas {
    pub fn create_layer(&self, rt: &mut RuntimeHandle, name: String) -> Result<Layer>
    pub fn count_points(&self, rt: &mut RuntimeHandle, layer: Arc<Layer>) -> Result<f64>
}
```

- **Rust → JS**:宏为对象生成 `impl IntoValue`,`into_value` 走
  `jsi_turbo::object::to_js`:建 prototype(方法表驱动,与模块共用
  `build_prototype`)→ `create_with_prototype` → NativeState 存
  `ModuleHolder`(复用,含 external_size 上报)→ debug `__type`。
- **JS → Rust**:参数类型 `Arc<T>`;宏生成 `impl FromValue for Arc<T>`,
  从 NativeState 还原同一 Arc——**引用语义,共享同一实例**。

## 明确的取舍(v1)

1. **JS 身号不同**:每次 `into_value` 新建 JS 包装对象,
   `canvas.createLayer('a') !== canvas.createLayer('a')`(即使背后同一
   Rust 实例)。Nitro 用 per-runtime WeakObject 缓存保证 `===`。
   v1 不做——身份判定用 Rust 侧(`countPoints(layer)` 数得对就是同一
   个);优化路径:per-runtime `(runtime ptr, table ptr) → JsiObject`
   缓存,runtime 销毁时失效(需要 teardown 钩子,与回调失效协议同源,
   见 ROADMAP JS 回调一节)。
2. **prototype 按次构建**:v1 每次交对象给 JS 都重建 prototype
   (O(方法数) 个 host fn)。对「偶尔创建对象」的场景够用;高 churn
   场景需要上面那个缓存。测试进程会顺序创建/销毁多个 runtime,
   全局缓存若只按 table ptr 键控会踩到地址复用,故 v1 不缓存。
3. **对象方法仅 sync**:async 方法需要 Dispatcher,而对象包装点
   (IntoValue)拿不到它。模块的 async 走 invoker 里的 dispatcher;
   对象要支持需把 dispatcher 经 TLS 或 per-runtime 注册表传递,
   推迟到与缓存一起做。宏对 `#[TurboObject]` 里的 async fn 直接
   报编译错误。
4. **TS 接口与 TurboStruct 同目录**(`generated/rust-turbo/structs/`):
   ts_type_of 对未知命名类型统一按 struct 导入,无法区分两种宏,
   v1 共用目录,文件头注释标注来源宏。

## 不变的保证

- 方法不可重赋(non-writable/non-configurable,与模块一致)
- 析构 `this` 调用 → 友好报错(共享 make_entry_fn)
- `turbo_external_size` 上报(共享 ModuleHolder)
- debug `__type = "RustTurboObject<Name>"`
