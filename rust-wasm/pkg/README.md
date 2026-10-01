# Image Compressor

这是一个基于Rust和WebAssembly的图像压缩工具，支持JPEG和PNG格式的图像压缩。

## 功能

- JPEG图像压缩：支持质量参数设置和4:2:0色度子采样
- PNG图像压缩：支持有损和无损压缩
- JPEG质量估计：可以估计JPEG图像的质量因子（实验性功能）

## 新增功能

### JPEG质量估计

我们添加了一个新的函数`estimate_jpeg_quality`，它可以估计JPEG图像的质量因子：

```rust
/// 估计JPEG图像的质量因子
/// 返回值范围为1-100，如果无法估计则返回0
pub fn estimate_jpeg_quality(input: &[u8]) -> u8
```

注意：由于`jpeg-decoder`库的`quantization_tables`字段是私有的，我们无法直接访问它来准确计算JPEG质量。因此，当前的实现返回一个默认值50。

在实际应用中，我们可以考虑以下几种解决方案：
1. 修改`jpeg-decoder`库以公开访问量化表的方法
2. 使用其他库来读取JPEG量化表
3. 实现自己的JPEG量化表解析器

## 安装依赖

```bash
cargo build
```

## 运行测试

```bash
cargo test
```