//! 単眼の深度推定 (`docs/ai-integration.md#深度推定による振り向きリグ`)。
//!
//! 深度推定はふつう画像から深度を出す専用のモデル (Depth Anything など) で行い、言語モデルの
//! [`crate::Provider`] では得られない。そこで推定器を [`DepthEstimator`] トレイトの裏に隠し、
//! 外部の推論サーバ ([`HttpDepthEstimator`])・ローカルの推定器・録画した深度マップ
//! ([`FixedDepthEstimator`]) を差し替えられるようにする。
//!
//! 推定器が返すのは **相対的な深度** ([`DepthMap`]、値が大きいほど手前) で、尺度とずれは決まらない。
//! キャンバスのピクセル単位の深度に直すのは振り向きの提案 ([`crate::turn`]) の役目。

use std::fmt;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::transport::{HttpRequest, Transport, UreqTransport, is_loopback_url};
use crate::{AiError, Destination, Disclosure};

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// 画素ごとの相対的な深度。値が大きいほど手前 (視差・逆深度と同じ向き)。
///
/// 尺度とずれは推定器ごとに違い、意味を持たない (順序と比だけを使う)。
/// 並びは行優先 (x が速い) で、長さは `width * height`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthMap {
    pub width: u32,
    pub height: u32,
    pub values: Vec<f32>,
}

impl DepthMap {
    /// 大きさと値の数が合い、値がすべて有限であることを確かめて作る。
    pub fn new(width: u32, height: u32, values: Vec<f32>) -> Result<Self, AiError> {
        let map = Self {
            width,
            height,
            values,
        };
        map.check()?;
        Ok(map)
    }

    /// 大きさと値の数が合い、値がすべて有限であるか。
    pub fn check(&self) -> Result<(), AiError> {
        let expected = self.width as usize * self.height as usize;
        if self.width == 0 || self.height == 0 {
            return Err(AiError::InvalidResponse("the depth map is empty".into()));
        }
        if self.values.len() != expected {
            return Err(AiError::InvalidResponse(format!(
                "the depth map is {}x{} but has {} values",
                self.width,
                self.height,
                self.values.len()
            )));
        }
        if let Some(i) = self.values.iter().position(|v| !v.is_finite()) {
            return Err(AiError::InvalidResponse(format!(
                "depth value {i} is not finite"
            )));
        }
        Ok(())
    }

    /// 画素 `(x, y)` の値。範囲の外なら `None`。
    pub fn get(&self, x: u32, y: u32) -> Option<f32> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.values
            .get(y as usize * self.width as usize + x as usize)
            .copied()
    }

    /// 画像全体を `0..1` とした座標 `(u, v)` の値を、画素の中心を結ぶ双線形補間で求める。
    /// 端の外は端の画素の値にする。
    pub fn sample(&self, u: f32, v: f32) -> f32 {
        let axis = |t: f32, n: u32| {
            let x = (t * n as f32 - 0.5).clamp(0.0, (n - 1) as f32);
            let i = (x.floor() as u32).min(n.saturating_sub(2));
            (i, (i + 1).min(n - 1), x - i as f32)
        };
        let (x0, x1, fx) = axis(u, self.width);
        let (y0, y1, fy) = axis(v, self.height);
        let at = |x: u32, y: u32| self.get(x, y).unwrap_or_default();
        let top = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * fx;
        let bottom = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * fx;
        top + (bottom - top) * fy
    }

    /// 向きを反転する (値が大きいほど奥の出力を、手前が大きい向きに直す)。
    pub fn inverted(mut self) -> Self {
        for v in &mut self.values {
            *v = -*v;
        }
        self
    }
}

/// 深度推定の入力。
#[derive(Debug, Clone, Default)]
pub struct DepthRequest {
    /// 原画 (PSD を合成した画像など) の PNG バイト列。画像はキャンバス全体に対応する。
    pub image: Vec<u8>,
}

/// 画像から相対的な深度マップを推定するもの。
///
/// 返す深度マップの大きさは画像と違ってよい (呼び出し側がキャンバスに引き伸ばして使う)。
pub trait DepthEstimator {
    fn estimate(&self, request: &DepthRequest) -> Result<DepthMap, AiError>;

    /// `request` を送るときに、どこへ何が出ていくか ([`crate::Provider::disclosure`] と同じ)。
    /// 既定はローカルで完結する扱い。
    fn disclosure(&self, request: &DepthRequest) -> Disclosure {
        disclosure(Destination::Local, request)
    }
}

fn disclosure(destination: Destination, request: &DepthRequest) -> Disclosure {
    Disclosure {
        destination,
        image_count: 1,
        image_bytes: request.image.len(),
        text_bytes: 0,
    }
}

/// 決めておいた深度マップを返す推定器。録画した深度マップでのテストや、ホストが別の手段で
/// 作った深度マップを渡すときに使う。
#[derive(Debug, Clone)]
pub struct FixedDepthEstimator {
    map: DepthMap,
}

impl FixedDepthEstimator {
    pub fn new(map: DepthMap) -> Result<Self, AiError> {
        map.check()?;
        Ok(Self { map })
    }
}

impl DepthEstimator for FixedDepthEstimator {
    fn estimate(&self, _request: &DepthRequest) -> Result<DepthMap, AiError> {
        Ok(self.map.clone())
    }
}

/// 推論サーバが返す深度の向き。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepthPolarity {
    /// 値が大きいほど手前 (視差・逆深度。Depth Anything・MiDaS の相対深度の出力)。
    #[default]
    NearIsLarger,
    /// 値が大きいほど奥 (距離。メトリック深度の出力)。
    FarIsLarger,
}

/// [`HttpDepthEstimator`] の設定。
#[derive(Debug, Clone)]
pub struct HttpDepthConfig {
    /// 送り先の URL (例: `http://127.0.0.1:8765/depth`)。
    pub endpoint: String,
    /// 利用者に示すモデルの名前 (例: `depth-anything-v2-small`)。
    pub model: String,
    pub polarity: DepthPolarity,
    pub timeout: std::time::Duration,
}

impl HttpDepthConfig {
    pub fn new(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            model: model.into(),
            polarity: DepthPolarity::default(),
            timeout: std::time::Duration::from_secs(120),
        }
    }

    /// 送り先がこの計算機 (ループバック) か。
    fn is_loopback(&self) -> bool {
        is_loopback_url(&self.endpoint)
    }
}

/// HTTP の推論サーバで深度を推定する。
///
/// 約束 (`docs/ai-integration.md#深度推定による振り向きリグ`):
/// - リクエスト: `POST <endpoint>`、本文は
///   `{"image": {"type": "base64", "media_type": "image/png", "data": "..."}}`。
/// - 応答: `{"width": W, "height": H, "depth": [W*H 個の数]}` (行優先)。向きは
///   [`HttpDepthConfig::polarity`] で指定する。
///
/// 送り先がループバックでなければ外部に送る扱い ([`Destination::External`]) にする。
pub struct HttpDepthEstimator {
    config: HttpDepthConfig,
    transport: Box<dyn Transport>,
}

impl fmt::Debug for HttpDepthEstimator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpDepthEstimator")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
struct DepthResponse {
    width: u32,
    height: u32,
    depth: Vec<f32>,
}

impl HttpDepthEstimator {
    pub fn new(config: HttpDepthConfig) -> Self {
        let transport = UreqTransport::new(config.timeout);
        Self::with_transport(config, transport)
    }

    /// 通信を差し替えた推定器 (テストでは [`crate::transport::ReplayTransport`] を渡す)。
    pub fn with_transport(config: HttpDepthConfig, transport: impl Transport + 'static) -> Self {
        Self {
            config,
            transport: Box::new(transport),
        }
    }

    pub fn config(&self) -> &HttpDepthConfig {
        &self.config
    }
}

impl DepthEstimator for HttpDepthEstimator {
    fn estimate(&self, request: &DepthRequest) -> Result<DepthMap, AiError> {
        if !request.image.starts_with(PNG_SIGNATURE) {
            return Err(AiError::InvalidRequest("the image is not a PNG".into()));
        }
        let data = base64::engine::general_purpose::STANDARD.encode(&request.image);
        let body = json!({
            "image": { "type": "base64", "media_type": "image/png", "data": data },
        });
        let http = HttpRequest {
            method: "POST".into(),
            url: self.config.endpoint.clone(),
            headers: vec![("content-type".into(), "application/json".into())],
            body: body.to_string().into_bytes(),
        };
        let response = self
            .transport
            .send(&http)
            .map_err(|e| AiError::Network(e.message))?;
        if !(200..300).contains(&response.status) {
            return Err(AiError::Api {
                status: response.status,
                kind: "depth_server_error".into(),
                message: String::from_utf8_lossy(&response.body)
                    .chars()
                    .take(500)
                    .collect(),
            });
        }
        let parsed: DepthResponse = serde_json::from_slice(&response.body)
            .map_err(|e| AiError::InvalidResponse(format!("depth response: {e}")))?;
        let map = DepthMap::new(parsed.width, parsed.height, parsed.depth)?;
        Ok(match self.config.polarity {
            DepthPolarity::NearIsLarger => map,
            DepthPolarity::FarIsLarger => map.inverted(),
        })
    }

    fn disclosure(&self, request: &DepthRequest) -> Disclosure {
        if self.config.is_loopback() {
            return disclosure(Destination::Local, request);
        }
        disclosure(
            Destination::External {
                service: "depth estimation server".into(),
                endpoint: self.config.endpoint.clone(),
                model: self.config.model.clone(),
            },
            request,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_map_checks_its_shape() {
        assert!(DepthMap::new(2, 2, vec![0.0; 4]).is_ok());
        assert!(DepthMap::new(2, 2, vec![0.0; 3]).is_err());
        assert!(DepthMap::new(0, 0, vec![]).is_err());
        assert!(DepthMap::new(1, 1, vec![f32::NAN]).is_err());
        let map = DepthMap::new(2, 1, vec![1.0, 2.0]).unwrap();
        assert_eq!(map.get(1, 0), Some(2.0));
        assert_eq!(map.get(2, 0), None);
        assert_eq!(map.inverted().values, vec![-1.0, -2.0]);
    }

    #[test]
    fn sampling_interpolates_between_pixel_centers() {
        let map = DepthMap::new(2, 2, vec![0.0, 1.0, 2.0, 3.0]).unwrap();
        // 画素の中心は 0.25 と 0.75。
        assert!((map.sample(0.25, 0.25) - 0.0).abs() < 1e-6);
        assert!((map.sample(0.75, 0.75) - 3.0).abs() < 1e-6);
        assert!((map.sample(0.5, 0.5) - 1.5).abs() < 1e-6);
        // 外は端の値。
        assert!((map.sample(-1.0, 0.25) - 0.0).abs() < 1e-6);
        assert!((map.sample(2.0, 0.75) - 3.0).abs() < 1e-6);
        let one = DepthMap::new(1, 1, vec![5.0]).unwrap();
        assert!((one.sample(0.3, 0.9) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn loopback_endpoints_stay_local() {
        let local = |url: &str| HttpDepthConfig::new(url, "m").is_loopback();
        assert!(local("http://127.0.0.1:8765/depth"));
        assert!(local("http://localhost/depth"));
        assert!(local("http://[::1]:80/"));
        assert!(!local("https://depth.example.com/v1"));
        assert!(!local("https://localhost.example.com/"));
    }
}
