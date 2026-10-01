[English](./README.md) | **简体中文**

# image_compressor WASM 图片压缩工具

## 简介

纯前端图片压缩工具，通过调整**质量（quality）**和**目标格式（jpeg/png）**缩小图片文件体积，压缩过程在浏览器本地完成，不上传原图、不占用服务端资源。

- 使用 Rust + `wasm-bindgen` 编译，底层编解码基于 Rust [`image` 0.24.9](https://crates.io/crates/image) crate
- 解码（输入）支持：PNG、JPEG、WebP、GIF、BMP、TIFF、ICO 等常见格式
- 编码（输出）仅支持：**JPEG、PNG**
- 压缩为同步 CPU 计算，在调用线程内阻塞执行

## 实现要点

- **JPEG**：按 `quality` 参数编码，固定使用 4:2:0 色度子采样，并启用优化的 Huffman 表以获得更好的压缩率
- **PNG**：`quality < 100` 时先用 imagequant 做调色板量化（有损）；`quality = 100` 输出无损 PNG
- 解码交给 [`image`](https://crates.io/crates/image) crate，因此常见输入格式会先被解码、再重新编码为目标格式

## 文件说明

npm 包发布的是 wasm-pack 生成的三个构建产物（仓库中位于 `rust-wasm/pkg/`）：

| 文件 | 作用 |
| --- | --- |
| `image_compressor.js` | wasm-bindgen 生成的 JS 胶水代码，负责加载 wasm、JS 与 wasm 之间的内存拷贝 |
| `image_compressor_bg.wasm` | 实际的压缩逻辑二进制文件，需与 `.js` 放在同一目录 |
| `image_compressor.d.ts` | wasm-bindgen 生成的 TypeScript 类型声明 |

> 胶水代码默认通过 `new URL('image_compressor_bg.wasm', import.meta.url)` 加载二进制。本项目是 Vue CLI 5（webpack 5），webpack 会自动识别该语法并把 wasm 作为静态资源发布，**不要手动改写加载路径**，也不要只拷贝其中一个文件。

## API

### `default export: init(module_or_path?)`

初始化（加载并实例化）wasm，返回 Promise，**必须在调用压缩函数前 `await` 完成**。

- 不传参数：自动 fetch 同目录下的 `image_compressor_bg.wasm`（推荐）
- 传入 `ArrayBuffer/Buffer`：跳过 fetch，直接实例化（Node 环境使用）
- 初始化只需执行一次，重复调用直接返回已实例化的模块

### `initSync(module)`

同步版本初始化，需要自行提供 wasm 二进制（`WebAssembly.Module` 或 ArrayBuffer）。

### `compress_image(input, quality, format): Uint8Array`

执行压缩。

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `input` | `Uint8Array` | 原始图片文件的二进制内容（`file.arrayBuffer()` 转换） |
| `quality` | `number` | 质量，**有效范围 1–100 的整数**，值越小体积越小、画质越低 |
| `format` | `string` | 输出格式，只接受 `'jpeg'` 或 `'png'` |
| 返回值 | `Uint8Array` | 压缩后的图片二进制；失败时返回**空数组（length = 0），不抛异常** |

## 使用示例

```bash
npm install img-compressor-wasm
```

```ts
// 1. 动态加载（可按需加载，wasm 不打进主包）
const { default: init, compress_image } = await import('img-compressor-wasm')

// 2. 初始化（只需一次，可用 window 变量或模块单例缓存）
await init()

// 3. File -> Uint8Array
const arrayBuffer = await file.arrayBuffer()
const uint8Array = new Uint8Array(arrayBuffer)

// 4. 压缩
const compressed: Uint8Array = compress_image(uint8Array, 50, 'jpeg')

// 5. 必须判空：不支持的输出格式 / 损坏的图片 / 解码失败都返回空数组
if (!compressed.length) {
  throw new Error('图片压缩失败')
}

// 6. Uint8Array -> Blob/File，用于预览或上传
const blob = new Blob([compressed], { type: 'image/jpeg' })
const url = URL.createObjectURL(blob)              // 预览
const newFile = new File([blob], file.name, { type: 'image/jpeg' }) // 上传
```

## 实测压缩数据

使用本项目 `src/assets` 中的图片实测（2026-10）：

**logo.png（原图 6.69 KB）**

| 输出 | 质量 80 | 质量 50 | 质量 20 |
| --- | --- | --- | --- |
| JPEG | 2.44 KB（-63.5%） | 1.71 KB（-74.4%） | 1.21 KB（-81.9%） |
| PNG  | 2.08 KB（-68.9%） | 1.84 KB（-72.4%） | 1.63 KB（-75.6%） |

**default.png（原图 4.07 KB）**

| 输出 | 质量 80 | 质量 50 | 质量 20 |
| --- | --- | --- | --- |
| JPEG | 5.18 KB（**+27.4%**） | 3.74 KB（-8.1%） | 2.54 KB（-37.5%） |
| PNG  | 2.00 KB（-50.7%） | 1.81 KB（-55.6%） | 1.91 KB（-53.1%） |

> 注意：小图 + 高质量转 JPEG 时，体积反而可能变大。压缩后应比较前后大小，或直接使用项目 UI 中展示的"压缩比"让用户选择。

## 参数边界行为（实测）

以下均为实测结果，业务代码不要依赖这些隐式行为，应自行保证入参合法：

- `quality` 按 `u8` 处理：`>100` 会被当作 100；负数、`256` 等会因字节截断得到意外值（如 `-10` 等价于 100、`256` 等价于 0）。**请在调用前自行 `Math.min(100, Math.max(1, Math.round(quality)))`**
- `format` 传 `'webp'`、`'gif'` 等不支持的输出格式：返回空 `Uint8Array`，不抛错
- 输入不是合法图片（或含无法解码的内容）：返回空 `Uint8Array`，不抛错
- 因此**每次调用后都必须检查返回值长度**

## 注意事项

1. **必须先初始化**：未 `await init()` 完成就调用 `compress_image` 会报错。
2. **失败不抛异常**：统一通过返回空数组表示失败，调用方需显式判空并提示用户。
3. **JPEG 不支持透明通道**：带透明背景的 PNG 转 JPEG 后透明区域会丢失（按图片自带 RGB 值呈现，通常为黑色/杂色），需要透明效果时输出 PNG。
4. **同步阻塞主线程**：压缩大图时可能造成页面卡顿。如需处理超大图片，建议放到 Web Worker 中调用。
5. **压缩不改变尺寸**：本工具只调整编码质量，输出图片的宽高与原图一致，不做缩放。

## 从源码构建

Rust 源码在 [`rust-wasm/`](./rust-wasm)。

用 wasm-pack 把 WebAssembly 产物构建到 `rust-wasm/pkg/`（需要 [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) 和 `wasm32-unknown-unknown` target）：

```bash
npm run build   # wasm-pack build rust-wasm --target web
```

包使用的三个文件 —— `image_compressor.js`、`image_compressor_bg.wasm`、`image_compressor.d.ts` —— 直接读取自 `rust-wasm/pkg/`。`prepublishOnly` 会在 `npm publish` 前自动执行构建，因此 `pkg/` 无需提交（已被 `rust-wasm/pkg/.gitignore` 忽略）。

单独检查 Rust 代码：

```bash
cd rust-wasm
cargo build   # 编译
cargo test    # 跑测试
```