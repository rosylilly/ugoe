//! 録画した深度マップから振り向きの提案を作り、適用したモデルの JSON を書き出す (目視確認用)。
//!
//! ```sh
//! cargo run -q -p ugoe-ai --example depth_turn -- /tmp/turn.json
//! cargo run -q -p ugoe-cli -- render /tmp/turn.json --param AngleX=30 -o /tmp/turn-x30.png
//! ```
//!
//! モデルは `tests/common/mod.rs` の顔 (後ろ髪・顔・目・鼻・口・前髪)、深度マップは
//! `tests/cassettes/depth_face.json` の録画を再生して得る。

#[path = "../tests/common/mod.rs"]
mod common;

use ugoe_ai::depth::{DepthRequest, HttpDepthConfig, HttpDepthEstimator};
use ugoe_ai::transport::ReplayTransport;
use ugoe_ai::turn::{TurnTarget, propose_depth_turn};
use ugoe_editor_core::{Command, Document};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: depth_turn <output.json>")?;
    let model = common::face_model();
    let estimator = HttpDepthEstimator::with_transport(
        HttpDepthConfig::new("http://127.0.0.1:8765/depth", "depth-anything-v2-small"),
        ReplayTransport::new(common::cassette("depth_face.json")),
    );
    let proposal = propose_depth_turn(
        &model,
        &estimator,
        &DepthRequest {
            image: common::png(),
        },
        &common::face_labels(),
        &[TurnTarget {
            deformer: "Face".into(),
            role: None,
        }],
    )?;
    eprintln!("{}\n{}", proposal.title, proposal.rationale);
    for command in &proposal.commands {
        if let Command::SetDepth {
            depth: Some(depth), ..
        } = command
        {
            for row in depth.chunks(5) {
                let row: Vec<_> = row.iter().map(|d| format!("{d:6.1}")).collect();
                eprintln!("  depth {}", row.join(" "));
            }
        }
    }
    let mut doc = Document::new(model);
    doc.apply(Command::Batch {
        label: proposal.title.clone(),
        commands: proposal.commands,
    })?;
    std::fs::write(&path, serde_json::to_string_pretty(doc.model())?)?;
    Ok(())
}
