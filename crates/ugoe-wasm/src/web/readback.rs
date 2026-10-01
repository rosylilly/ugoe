//! `WebPuppet` の描画結果の読み戻しと、評価の結果 (不透明度・描画順の値など) の取り出し (T0418)。
//!
//! 適合テスト (`tests/conformance/README.md`) が WebGPU でも画像を比べるために使う。canvas の読み戻し
//! (`drawImage`・`toBlob`) はブラウザによって空になることがあるので、canvas とは別のテクスチャに描いて
//! バッファに写し、マップして読む。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use super::{WebPuppet, js_error};
use crate::readback;

#[wasm_bindgen]
extern "C" {
    // window とワーカーのどちらにもある大域の関数。
    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(callback: &js_sys::Function, ms: i32) -> JsValue;
}

/// マップの完了を待つ間隔 (ミリ秒) と回数の上限 (合わせて約 1 分)。
const WAIT_MS: i32 = 4;
const WAIT_LIMIT: u32 = 15_000;

#[wasm_bindgen]
impl WebPuppet {
    /// 今の状態を canvas と同じ大きさ (`width()` × `height()`)・同じビュー・同じ背景で canvas とは別のテクスチャに描き、
    /// 画素を読み戻す。値はストレートアルファの RGBA (各 8 ビット)、行は上から下。canvas には描かない。
    ///
    /// 読み戻しは非同期なので、`Promise` が解決するまでの間に `WebPuppet` を操作してもよい (結果は呼んだときの状態)。
    #[wasm_bindgen(unchecked_return_type = "Promise<Uint8Array>")]
    pub fn read_pixels(&mut self) -> Result<js_sys::Promise, JsError> {
        if self.is_lost() {
            return Err(JsError::new("the GPU device was lost"));
        }
        self.evaluate();
        let (width, height) = (self.config.width, self.config.height);
        let format = self.view_format;
        let bgra = match format {
            wgpu::TextureFormat::Rgba8Unorm => false,
            wgpu::TextureFormat::Bgra8Unorm => true,
            other => {
                return Err(JsError::new(&format!(
                    "reading back {other:?} is not supported"
                )));
            }
        };
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ugoe web readback target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ugoe web readback"),
            });
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.background),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        if let (Some(puppet), Some(transform)) = (&self.puppet, self.transform()) {
            self.renderer
                .encode(
                    &self.device,
                    &self.queue,
                    &mut encoder,
                    &view,
                    &transform.to_view([width, height]),
                    puppet.instance(),
                )
                .map_err(js_error)?;
        }
        // 読み戻しバッファの行は COPY_BYTES_PER_ROW_ALIGNMENT の倍数に揃える必要がある。
        let row_bytes = width * 4;
        let padded = row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ugoe web readback"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            target.size(),
        );
        self.queue.submit([encoder.finish()]);

        let mapped: Rc<RefCell<Option<Result<(), wgpu::BufferAsyncError>>>> = Rc::default();
        let slot = mapped.clone();
        buffer.map_async(wgpu::MapMode::Read, .., move |r| {
            *slot.borrow_mut() = Some(r);
        });
        let device = self.device.clone();
        let lost = self.lost.clone();
        Ok(wasm_bindgen_futures::future_to_promise(async move {
            // WebGPU はブラウザがマップを終えるとコールバックを呼ぶ。WebGL2 は poll の中で呼ぶ。
            // どちらでも、イベントループに戻りながら終わるまで待つ。
            let mut waited = 0;
            let result = loop {
                let _ = device.poll(wgpu::PollType::Poll);
                if let Some(r) = mapped.borrow_mut().take() {
                    break r;
                }
                if lost.load(Ordering::Relaxed) {
                    return Err(JsError::new("the GPU device was lost").into());
                }
                waited += 1;
                if waited > WAIT_LIMIT {
                    return Err(JsError::new("timed out reading back the pixels").into());
                }
                sleep(WAIT_MS).await;
            };
            result.map_err(|e| JsValue::from(js_error(e)))?;
            let range = buffer
                .get_mapped_range(..)
                .map_err(|e| JsValue::from(js_error(e)))?;
            let mut pixels = Vec::with_capacity(row_bytes as usize * height as usize);
            for row in range.chunks_exact(padded as usize) {
                pixels.extend_from_slice(&row[..row_bytes as usize]);
            }
            drop(range);
            buffer.unmap();
            readback::to_straight_rgba(&mut pixels, bgra);
            Ok(js_sys::Uint8Array::from(pixels.as_slice()).into())
        }))
    }

    /// 要素ごとの評価の結果の JSON (`drawable_ids()` の順)。各要素は
    /// `{"opacity", "draw_order", "multiply_color": [r, g, b], "screen_color": [r, g, b], "mask_strength", "visible"}`
    /// (`docs/spec/evaluation.md#8-出力`)。頂点の位置は `drawable_positions`、描画の順番は `render_order`。モデルが無ければ `[]`。
    pub fn drawable_states_json(&mut self) -> Result<String, JsError> {
        self.evaluate();
        let states = self
            .puppet
            .as_ref()
            .map(|p| readback::drawable_states(p.instance()))
            .unwrap_or_default();
        serde_json::to_string(&states).map_err(js_error)
    }
}

/// `ms` ミリ秒後に解決する (イベントループに戻る)。
async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        set_timeout(&resolve, ms);
    });
    let _ = JsFuture::from(promise).await;
}
