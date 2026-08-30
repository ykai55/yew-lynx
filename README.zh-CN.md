# Lynx Element Bridge

[English](README.md) | 简体中文

> [!WARNING]
> **实验性公开预览。** 这是一个独立研究项目，并非 Lynx、Yew 或 Dioxus
> 官方支持的集成方案。兼容性仅限于下文列出的固定版本和已验证目标。

Lynx Element Bridge 用于将 Rust UI 框架直接挂载到 Lynx Fiber DOM。Yew 和
Dioxus 会渲染为共享的 Rust 变更模型，再通过带版本的原生 C 函数表应用这些
变更。应用生命周期不使用 Lynx 模板包、Java `LynxModule`、JavaScript/MTS，
也不使用序列化的原生命令通道。

仓库目前提供：

- 具有等价可观测行为的 Yew 和 Dioxus 适配器。
- 链接到 APK 中的 Android Native 运行时。
- 可加载外部 `wasm32-wasip1` 应用的 WAMR 运行时。
- 基于补丁实现、仅包含原生渲染器的 Lynx Android 产品。
- Host、ABI、集成、产物和物理设备验证。

## 架构

### 分层设计

```text
框架层
  Yew NativeRendererBackend          Dioxus WriteMutations
                \                    /
                 v                  v
桥接核心
  Session -> 有序 CommandBatch -> 树/listener 校验
                              |
执行后端                      |
  Native Rust ----------------+---------------- WAMR 托管的 Rust
                              |
原生边界                      v
  FFI session registry -> NativeHost -> LynxNativeRendererApiV1
                                             |
Lynx 平台层                                  v
                                      Lynx Fiber DOM
```

主要边界被有意设计得尽量窄：

1. **框架适配器生成命令。** Yew 和 Dioxus 将各自的渲染器变更转换为同一个
   与框架无关的 `CommandBatch`。
2. **核心层保证正确性。** `Session` 在命令到达 Lynx 之前校验节点所有权、
   树结构、listener 身份、变更顺序、线程所有权，并确保确定性清理。
3. **FFI 层保证生命周期安全。** 它管理不透明 session token、同步回调、
   重入拒绝、panic 隔离、部分原生操作失败后的中毒状态、正常销毁和紧急放弃。
4. **`NativeHost` 管理 Lynx 映射。** 它将 bridge ID 映射到不透明 Lynx
   handle，并通过复制且校验过的 `LynxNativeRendererApiV1` 函数表执行每个 batch。
5. **打过补丁的 Lynx 负责渲染。** 原生渲染器创建 Fiber 元素、应用变更、
   分发平台事件并刷新更新，全程不需要模板或 JavaScript 运行时。

`CommandBatch` 是 Rust 内存内的边界。Native 应用不会序列化命令。事件的
content type 和 payload 字节会在原生回调边界被复制，但对 bridge 始终保持
不透明。

### Native 与 WASM 模式

两种运行模式最终都会进入同一个 `NativeHost` 和 Lynx C API：

```text
Native
  Yew/Dioxus 应用 staticlib
      -> element-bridge-ffi
      -> NativeHost
      -> Lynx C API

WASM
  Yew/Dioxus wasm32-wasip1 guest
      -> FlatBuffers v3 guest ABI
      -> WAMR host
      -> element-bridge-ffi / NativeHost
      -> Lynx C API
```

序列化只存在于 WASM guest 和 WAMR host 之间。Android APK 包含与框架无关的
WAMR host，但不包含 guest `.wasm`；guest 会被单独构建并通过 URL 加载。除此
之外，Native 和 WASM session 使用相同的渲染器生命周期和安全模型。

### 渲染与事件流程

一次挂载或事件处理会形成一个在 owner thread 上同步执行的事务：

```text
框架渲染
  -> 适配器记录变更
  -> Session 校验并提交 CommandBatch
  -> NativeHost 应用命令并刷新 Lynx

平台事件
  -> Lynx 原生回调
  -> FFI 校验 session/listener/callback 身份
  -> 适配器分发框架回调
  -> 框架渲染下一个 CommandBatch
  -> NativeHost 应用并刷新
```

Lynx 接受 batch 中的部分命令后不会进行回滚。Host 部分失败会使 session 进入
中毒状态，防止后续调用在未知状态上继续执行。正常销毁会先渲染框架的清理变更，
然后释放 renderer；abandon 只消费 bridge 状态，仅用于紧急清理。

### 仓库地图

| 路径 | 职责 |
| --- | --- |
| `crates/element-bridge-core/` | 与框架无关的 ID、命令、事件、session 不变量和 `HostFake` |
| `adapters/yew/` | 将打过补丁的 Yew `NativeRendererBackend` 接入核心 `Session` |
| `adapters/dioxus/` | Dioxus `WriteMutations` 以及 Lynx 原生 `view`/`text` RSX 词汇 |
| `crates/adapter-conformance/` | 跨框架的挂载、更新、事件和销毁一致性测试 |
| `crates/element-bridge-ffi/` | Native session registry、C ABI 生命周期和 `NativeHost` |
| `crates/element-bridge-protocol/` | Host/Guest 共享的 FlatBuffers v3 schema、已签入 bindings、owned 类型和 codecs |
| `crates/element-bridge-wasm-guest/` | 带版本的 WASM guest ABI 和应用生命周期 |
| `crates/element-bridge-wamr-host/` | WAMR 嵌入以及 guest 到原生后端的集成 |
| `adapters/android/` | 通过 `dlsym` 解析 Lynx 函数表的 JNI/CMake bridge |
| `examples/counter/` | Yew Native 和 WASM counter 应用 |
| `examples/dioxus-counter/` | Dioxus Native 和 WASM counter 应用 |
| `examples/android/` | Android 启动器以及 Native/WASM runtime host |
| `tools/dev-wasm/` | WASM 构建、监听、HTTP 服务和 reload 通知 |
| `include/` | Lynx renderer、Native 应用和 WAMR 应用的公开 C ABI |
| `patches/lynx/` | 实现 Lynx 原生渲染器的有序补丁系列 |
| `patches/yew/` | 为 Yew 添加原生渲染器接口的有序补丁系列 |

实际生效的 Lynx 和 Yew 集成由固定的上游版本和这些补丁系列共同组成；不能假设
这些 API 在其他任意版本的上游项目中存在。

## 快速开始

### 1. 运行 Rust 测试

仓库在 `rust-toolchain.toml` 中固定使用 Rust `1.85.0`。初始化 submodule、
准备打过补丁的 Yew checkout，然后运行 host 侧 workspace 测试：

```bash
git submodule update --init --recursive
./scripts/bootstrap-yew.sh
cargo test --workspace --all-targets --locked
```

如需包含真实的嵌入式 WAMR 生命周期测试：

```bash
cargo test -p lynx-element-bridge-wamr-host --features wamr -- --test-threads=1
```

### 2. 构建并运行 Android 示例

目前支持的示例目标为 Android API 24+、`arm64-v8a`。脚本构建需要：

- JDK 11。
- Android SDK platform 33 和 build-tools 33.0.1。
- Android NDK 21.1.6352462（用于 Lynx）和 25.2.9519653（用于 Rust/JNI 链接）。
- CMake 3.22.1。
- Rust 1.85.0，并安装 `aarch64-linux-android` 和 `wasm32-wasip1` target。
- Node.js 22.18.0，用于准备 Lynx 源码。

设置 `ANDROID_HOME` 或 `ANDROID_SDK_ROOT`，然后从仓库根目录构建一个 Native
框架变体：

```bash
export ANDROID_HOME=/path/to/Android/sdk
./scripts/build-android.sh --backend yew
# 或者
./scripts/build-android.sh --backend dioxus
```

首次在线构建会初始化固定版本的源码、应用补丁系列、构建并发布本地 Lynx 产物、
链接 Native 和 WAMR bridge library、组装 APK，并检查其依赖和 ELF 边界。完成
一次准备后，后续构建可以添加 `--offline`；使用 `--clean` 可删除生成的集成产物。
这两个参数不能组合使用。

安装并打开所选 APK：

```bash
adb install -r .deps/android/apks/lynx-element-bridge-yew.apk
adb shell am start -n com.yew.lynx.example/.LauncherActivity
```

Dioxus 构建需要将 APK 文件名中的 `yew` 替换为 `dioxus`。启动页可以进入编译进
APK 的 Native counter，也可以进入外部 WASM 流程。Android Studio、离线构建和
设备验收说明请参阅
[`examples/android/README.md`](examples/android/README.md)。

### 3. 运行 WASM Guest

使用仓库提供的开发服务器构建并托管两个示例 guest：

```bash
./scripts/dev-wasm.sh
adb reverse tcp:8000 tcp:8000
```

在已安装的应用中选择 **WASM**，然后输入服务器输出的 URL，例如：

```text
http://127.0.0.1:8000/yew_lynx_counter.wasm
http://127.0.0.1:8000/lynx_element_bridge_dioxus_counter.wasm
```

使用 `./scripts/dev-wasm.sh --backend yew` 或 `--backend dioxus` 可只监听一个
guest。`--bind IP` 和 `--port PORT` 可修改监听地址。成功重新构建后，应用会校验
服务器公布的产物并重新挂载。Reload 会创建新的组件树，不会保留应用状态。

## 构建产物

Android 构建会严格分离 stock Lynx 产品和 native Lynx 产品：

| 产品 | 主要 library | 用途 |
| --- | --- | --- |
| `org.lynxsdk.lynx:lynx` | `liblynx.so` | 保持原样的 JavaScript/模板产品 |
| `org.lynxsdk.lynx:lynx-native-renderer` | `liblynx_native_renderer.so` | 本应用使用的可选原生 Fiber renderer |

每个示例 APK 都包含：

- `liblynx_element_bridge_native.so`，链接所选的 Yew 或 Dioxus Native runtime。
- `liblynx_element_bridge_wamr.so`，与框架无关的 WAMR host。
- `liblynx_native_renderer.so` 及其 Lynx 原生支持 library。

构建会拒绝 stock `liblynx.so`、Quick/PrimJS、NAPI、V8、LynxJSSDK 的
`assets/lynx_core.js`、打包的 `.lynx.bundle` 文件以及打包的 WASM guest。

## 验证

满足 Android 构建要求后，可运行完整的仓库验证：

```bash
./scripts/verify.sh
```

验证范围包括格式化、check、测试、Clippy、适配器一致性、公开 Native 生命周期
测试、真实 WAMR 生命周期测试、Android Java/JNI mock、Rust Android staticlib、
Lynx 和 Yew 补丁应用、公开 header 一致性，以及产物、依赖和 ELF 门禁。

当前固定并已验证的输入：

| 组件 | 版本或 revision |
| --- | --- |
| Lynx | `0df14207cebb060f1bed8de12b64a1119dee8f06` |
| Lynx tools_shared | `ff47fee7d41ee3e8e8561041b1ce2c8b50e923ea` |
| WAMR | `25bd7eb63e828e4bd242cc9b38d260b4b31c6605` |
| Yew patch base | `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3` |
| Dioxus | `0.7.10` |
| Rust | `1.85.0` |

不要推断版本固定值变化后的兼容性。完整支持矩阵、运行时契约、物理设备证据和已知
限制请参阅 [`COMPATIBILITY.md`](COMPATIBILITY.md)。如需添加其他 Rust UI 框架
适配器，请参阅 [`docs/adapter-authoring.md`](docs/adapter-authoring.md)。

## 当前限制

- 这是研究预览版本，不是可用于生产环境的运行时。
- 目前只支持并验证 Android API 24+ 的 `arm64-v8a`。
- 未覆盖 iOS、Harmony、桌面端、Web、无障碍和性能。
- 框架支持范围有意小于各框架的 Web renderer，不能假设未声明的 Yew/Dioxus
  功能可以工作。
- WASM reload 会替换完整的 guest/session，不保留组件状态。

## 许可证

本仓库采用 Apache-2.0 许可证。上游项目保留各自的许可条款，详情请参阅
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)。
