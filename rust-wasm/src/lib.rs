use image::{
    codecs::png::PngEncoder,
    GenericImageView, ImageEncoder,
};
use imagequant::{Attributes, Histogram, RGBA};
use std::io::Cursor;
use wasm_bindgen::prelude::*;
use web_sys::console;

// 添加png crate用于创建带调色板的PNG
use png::{BitDepth, ColorType as PngColorType, Encoder, EncodingError, Filter};

// 添加jpeg-encoder crate用于JPEG编码
use jpeg_encoder::{ColorType as JpegColorType, Encoder as JpegEncoder, SamplingFactor};

#[wasm_bindgen]
pub fn compress_image(input: &[u8], quality: u8, format: &str) -> Vec<u8> {
    // 尝试加载图像，如果失败则返回空的Vec
    let img = match image::load_from_memory(input) {
        Ok(img) => img,
        Err(_) => return vec![],
    };

    let mut output = Vec::new();

    match format {
        "jpeg" => {
            let mut cursor = Cursor::new(&mut output);
            // 确保质量值在有效范围内，避免极低质量值导致的问题
            let actual_quality = if quality == 0 { 1 } else { quality.min(100) };
            
            // 使用jpeg-encoder库替代image库的JpegEncoder
            // 创建JPEG编码器并设置4:2:0色度子采样
            let mut encoder = JpegEncoder::new(&mut cursor, actual_quality);
            encoder.set_sampling_factor(SamplingFactor::R_4_2_0); // 设置4:2:0色度子采样
            // 启用优化的Huffman表以获得更好的压缩率
            encoder.set_optimized_huffman_tables(true);
            
            // 将图像转换为RGB格式并编码
            let rgb_img = img.to_rgb8();
            let width = rgb_img.width();
            let height = rgb_img.height();
            
            // 编码图像，需要将u32转换为u16
            if encoder.encode(rgb_img.as_raw(), width as u16, height as u16, JpegColorType::Rgb).is_err() {
                return vec![];
            }
        }
        "png" => {
            // 使用imagequant实现PNG有损压缩
            if quality < 100 {
                let (width, height) = img.dimensions();
                let mut attrs = Attributes::new();
                // 根据quality参数设置压缩质量 (0-100映射到0-255)
                let quality_value = quality as f32; // / 100.0 * 255.0;
                                                    // 输出quality_value到浏览器控制台
                console::log_2(&"Quality value:".into(), &quality_value.into());
                // 如果设置质量失败，使用默认质量
                if attrs.set_quality(0, quality_value as u8).is_err() {
                    // 继续使用默认质量
                }

                // 强行指定颜色数量 (这里设置为quality的2倍，但不超过256)
                // if attrs.set_max_colors(16 as u32).is_err() {
                //     console::log_1(&"Error setting max colors".into());
                // }

                // 将图像转换为RGBA8格式
                let rgba_img = img.to_rgba8();
                let pixels: Vec<RGBA> = rgba_img
                    .pixels()
                    .map(|p| RGBA {
                        r: p[0],
                        g: p[1],
                        b: p[2],
                        a: p[3],
                    })
                    .collect();

                // 创建直方图并添加图像颜色
                let mut histogram = Histogram::new(&attrs);
                // 注意：width和height应该是usize类型
                if let Ok(mut quant_img) =
                    attrs.new_image(pixels, width as usize, height as usize, 0.0)
                {
                    if histogram.add_image(&attrs, &mut quant_img).is_ok() {
                        // 进行量化
                        if let Ok(mut result) = histogram.quantize(&attrs) {
                            // 输出直方图确定的颜色数量到浏览器控制台
                            console::log_2(
                                &"Palette colors count:".into(),
                                &result.palette().len().into(),
                            );
                            // 获取调色板和索引像素
                            if let Ok((palette, indexed_pixels)) = result.remapped(&mut quant_img) {
                                // 使用新的函数创建带调色板的PNG
                                match create_palette_png(
                                    &mut output,
                                    &palette,
                                    &indexed_pixels,
                                    width,
                                    height,
                                ) {
                                    Ok(_) => console::log_1(
                                        &"Image encoded successfully as palette mode".into(),
                                    ),
                                    Err(e) => {
                                        console::log_2(
                                            &"Error encoding image:".into(),
                                            &e.to_string().into(),
                                        );
                                        // 如果编码失败，回退到原始图像
                                        let mut cursor = Cursor::new(&mut output);
                                        let encoder = PngEncoder::new(&mut cursor);
                                        // 如果再次失败，返回空的Vec
                                        if img.write_with_encoder(encoder).is_err() {
                                            return vec![];
                                        }
                                    }
                                }
                            } else {
                                // 如果获取调色板和索引像素失败，则使用原始图像
                                let mut cursor = Cursor::new(&mut output);
                                let encoder = PngEncoder::new(&mut cursor);
                                // 如果编码失败，返回空的Vec
                                if img.write_with_encoder(encoder).is_err() {
                                    return vec![];
                                }
                            }
                        } else {
                            // 如果量化失败，则使用原始图像
                            let mut cursor = Cursor::new(&mut output);
                            let encoder = PngEncoder::new(&mut cursor);
                            // 如果编码失败，返回空的Vec
                            if img.write_with_encoder(encoder).is_err() {
                                return vec![];
                            }
                        }
                    } else {
                        // 如果添加图像到直方图失败，则使用原始图像
                        let mut cursor = Cursor::new(&mut output);
                        let encoder = PngEncoder::new(&mut cursor);
                        // 如果编码失败，返回空的Vec
                        if img.write_with_encoder(encoder).is_err() {
                            return vec![];
                        }
                    }
                } else {
                    // 如果创建QuantImage失败，则使用原始图像
                    let mut cursor = Cursor::new(&mut output);
                    let encoder = PngEncoder::new(&mut cursor);
                    // 如果编码失败，返回空的Vec
                    if img.write_with_encoder(encoder).is_err() {
                        return vec![];
                    }
                }
            } else {
                // 如果quality为100，则使用无损PNG压缩
                let mut cursor = Cursor::new(&mut output);
                let encoder = PngEncoder::new(&mut cursor);
                // 如果编码失败，返回空的Vec
                if img.write_with_encoder(encoder).is_err() {
                    return vec![];
                }
            }
        }
        _ => {
            // 不支持的格式，返回空的Vec
            return vec![];
        }
    }

    output
}

// 新增函数：创建带调色板的PNG
fn create_palette_png(
    output: &mut Vec<u8>,
    palette: &[RGBA],
    indexed_pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<(), EncodingError> {
    let mut cursor = Cursor::new(output);
    let mut encoder = Encoder::new(&mut cursor, width, height);

    // // 设置滤波类型为Sub
    // encoder.set_filter(Filter::Sub);
    
    // // 设置压缩级别为最高压缩比
    // encoder.set_compression(png::Compression::High);

    // 检查是否有透明像素
    let has_transparency = palette.iter().any(|color| color.a < 255);

    if has_transparency {
        console::log_1(&"Image has transparency, using Indexed+Alpha channel".into());
        // 如果有透明度，使用Indexed+Alpha通道
        encoder.set_color(PngColorType::Indexed);
        encoder.set_depth(BitDepth::Eight);

        // 设置调色板
        let mut palette_data = Vec::new();
        for color in palette {
            palette_data.push(color.r);
            palette_data.push(color.g);
            palette_data.push(color.b);
        }
        encoder.set_palette(palette_data);

        // 设置透明度数据
        let mut transparency_data = Vec::new();
        for color in palette {
            transparency_data.push(color.a);
        }
        encoder.set_trns(transparency_data);
    } else {
        console::log_1(&"Image has no transparency, using Indexed channel".into());
        // 如果没有透明度，使用普通的Indexed颜色类型
        encoder.set_color(PngColorType::Indexed);
        encoder.set_depth(BitDepth::Eight);

        // 设置调色板
        let mut palette_data = Vec::new();
        for color in palette {
            palette_data.push(color.r);
            palette_data.push(color.g);
            palette_data.push(color.b);
        }
        encoder.set_palette(palette_data);
    }

    let mut writer = encoder.write_header()?;
    writer.write_image_data(indexed_pixels)?;
    Ok(())
}
