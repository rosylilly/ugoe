//! 作成の途中や描画中に WebGL2 のコンテキストを失っても、glow・wgpu の中で panic しないようにする (T0944)。
//!
//! 失ったコンテキストの WebGL の呼び出しは `null` や既定の値を返す。glow (0.17) と wgpu-hal (30) は、そのうちいくつかを
//! 確かめずに `unwrap` するので、失った時機によっては wasm が panic する (panic の後は作成の Promise が決着しない)。
//! JS からの喪失 (`loseContext`) は同期的に進む作成に割り込めないが、GPU プロセスの喪失は割り込める
//! (`packages/player/browser-test` の `lose-context-during-create`・`gpu-process-crash-during-create`)。
//!
//! そこで、そうした呼び出しだけを、そのコンテキストのオブジェクトの上で包み (自分のプロパティにする。プロトタイプは
//! 変えない)、panic する値が返ったら panic しない値に置き換える。置き換えるのは、生きているコンテキストでは返らない値
//! (返れば今も panic する値) だけなので、生きているコンテキストの振る舞いは変わらない。失ったコンテキストに何を描いても
//! 見えないので、置き換えた値が本物でなくても害はない (`WebPuppet` は `webglcontextlost` で `is_lost()` になり、
//! ホストが作り直す)。`isContextLost()` では見分けない: GPU プロセスが落ちた直後は、呼び出しが既定の値を返しても
//! `isContextLost()` は偽のまま (`getError()` も 0) のことがある (Chrome 154 で確かめた)。
//!
//! 包むもの (見つけた panic の場所):
//! - `getSupportedExtensions()` が `null` → 空の配列 (glow の `from_webgl2_context` が `unwrap` する)。
//! - `getParameter(VERSION)` が文字列でない → `"WebGL 2.0"` (同上。版の文字列を読めないと panic する)。
//! - `getUniformBlockIndex()` が `INVALID_INDEX` → どのプログラムにも無い番号 (wgpu-hal の `create_render_pipeline` が
//!   `unwrap` する。GPU プロセスが落ちると、リンクできたことを読んだ後に `INVALID_INDEX` が返ることがある)。
//!   続く `uniformBlockBinding` は `INVALID_VALUE` で何もしない。
//!
//! 上流 (glow・wgpu-hal) が失ったコンテキストを確かめるようになったら、このモジュールは消せる (T0475 のメモの上流への報告の案)。

use js_sys::{Array, Function, Reflect};
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext;

/// `getUniformBlockIndex` の「無い」(`GL_INVALID_INDEX`)。
const INVALID_INDEX: u32 = u32::MAX;
/// `INVALID_INDEX` の代わりに返す番号 (ユニフォームブロックの数の上限より大きいので、どのプログラムにも無い)。
const NO_BLOCK_INDEX: u32 = u32::MAX - 1;

/// 包んだ関数 (元の関数の最初の 2 つの引数を受け取る)。
type Wrapper = Closure<dyn Fn(JsValue, JsValue) -> JsValue>;

/// コンテキストに付けた包み。捨てると外す (`WebPuppet` より先には捨てない。描画中に呼ばれるため)。
#[derive(Debug)]
pub(super) struct LostContextGuard {
    gl: WebGl2RenderingContext,
    wrappers: Vec<(&'static str, Wrapper)>,
}

impl LostContextGuard {
    /// `gl` の上で包む。プロパティを置けなければ (起きないはず) 包まずに続ける。
    pub(super) fn install(gl: &WebGl2RenderingContext) -> Self {
        let mut guard = Self {
            gl: gl.clone(),
            wrappers: Vec::new(),
        };
        guard.wrap("getSupportedExtensions", |result, _| {
            result.is_null().then(|| Array::new().into())
        });
        guard.wrap("getParameter", |result, pname| {
            let version = pname.as_f64() == Some(f64::from(WebGl2RenderingContext::VERSION));
            (version && result.as_string().is_none()).then(|| JsValue::from_str("WebGL 2.0"))
        });
        guard.wrap("getUniformBlockIndex", |result, _| {
            (result.as_f64() == Some(f64::from(INVALID_INDEX)))
                .then(|| JsValue::from_f64(f64::from(NO_BLOCK_INDEX)))
        });
        guard
    }

    /// `name` を包む。`replace(結果, 最初の引数)` が `Some` なら、結果の代わりにその値を返す。
    fn wrap(&mut self, name: &'static str, replace: fn(&JsValue, &JsValue) -> Option<JsValue>) {
        let Ok(original) = Reflect::get(&self.gl, &name.into()) else {
            return;
        };
        let Ok(original) = original.dyn_into::<Function>() else {
            return;
        };
        let gl = self.gl.clone();
        let wrapper = Wrapper::new(move |a: JsValue, b: JsValue| -> JsValue {
            // 元の呼び出しの例外は、包まないときと同じく呼び手 (web-sys) へ返す。
            let result = match original.call2(&gl, &a, &b) {
                Ok(result) => result,
                Err(e) => wasm_bindgen::throw_val(e),
            };
            replace(&result, &a).unwrap_or(result)
        });
        if Reflect::set(&self.gl, &name.into(), wrapper.as_ref()).unwrap_or(false) {
            self.wrappers.push((name, wrapper));
        }
    }
}

impl Drop for LostContextGuard {
    /// 自分のプロパティを消して、プロトタイプの元の関数に戻す (捨てた包みが呼ばれないように)。
    fn drop(&mut self) {
        for (name, _) in &self.wrappers {
            let _ = Reflect::delete_property(&self.gl, &(*name).into());
        }
    }
}
