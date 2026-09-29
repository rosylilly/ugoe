//! テスト用のプロバイダ。

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::{AiError, Provider, ProviderRequest};

/// 決めておいた応答を順に返すプロバイダ。ネットワークを使わない。
///
/// 受け取ったリクエストとスキーマを記録するので、呼び出し側のテストで中身を確かめられる。
#[derive(Debug, Default)]
pub struct FakeProvider {
    responses: Mutex<VecDeque<Result<serde_json::Value, AiError>>>,
    calls: Mutex<Vec<(ProviderRequest, serde_json::Value)>>,
}

impl FakeProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// 応答を 1 つ積む (呼ばれた順に返す)。
    pub fn push(&self, response: serde_json::Value) -> &Self {
        self.push_result(Ok(response))
    }

    /// エラーを含む応答を 1 つ積む。
    pub fn push_result(&self, response: Result<serde_json::Value, AiError>) -> &Self {
        lock(&self.responses).push_back(response);
        self
    }

    /// これまでに受け取ったリクエストとスキーマ。
    pub fn calls(&self) -> Vec<(ProviderRequest, serde_json::Value)> {
        lock(&self.calls).clone()
    }
}

impl Provider for FakeProvider {
    fn complete_json(
        &self,
        request: &ProviderRequest,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, AiError> {
        lock(&self.calls).push((request.clone(), schema.clone()));
        lock(&self.responses).pop_front().unwrap_or_else(|| {
            Err(AiError::Provider(
                "FakeProvider has no more responses".into(),
            ))
        })
    }
}

/// 毒化したロックもそのまま使う (中身は単純なキューで、壊れた状態にならない)。
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn returns_responses_in_order_and_records_calls() {
        let fake = FakeProvider::new();
        fake.push(json!({"a": 1})).push(json!({"b": 2}));
        let req = ProviderRequest {
            prompt: "hi".into(),
            ..Default::default()
        };
        assert_eq!(
            fake.complete_json(&req, &json!({})).unwrap(),
            json!({"a": 1})
        );
        assert_eq!(
            fake.complete_json(&req, &json!({})).unwrap(),
            json!({"b": 2})
        );
        assert!(fake.complete_json(&req, &json!({})).is_err());
        assert_eq!(fake.calls().len(), 3);
        assert_eq!(fake.calls()[0].0.prompt, "hi");
        assert!(!fake.disclosure(&req).is_external());
    }
}
