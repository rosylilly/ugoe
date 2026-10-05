//! PSD を取り込み、レイヤー名の規則で分類して、テンプレートのリグの提案を適用したバンドルを書き出す (目視確認用)。
//!
//! ```sh
//! cargo run -q -p ugoe-ai --example auto_rig -- assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd /tmp/rig.ugoe
//! cargo run -q -p ugoe-cli -- render /tmp/rig.ugoe --param EyeLOpen=0 -o /tmp/rig-eye.png
//! ```
//!
//! AI は使わない (値は `RigParams::default()`、深度は顔に内接する楕円体)。

use ugoe_ai::classify::classify_by_name;
use ugoe_ai::rig::{RigOptions, RigParams, mesh_parts, propose_rig};
use ugoe_editor_core::{Command, Project, PsdImportOptions, export_bundle, import_psd};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(output)) = (args.next(), args.next()) else {
        return Err("usage: auto_rig <input.psd> <output.ugoe>".into());
    };
    let bytes = std::fs::read(&input)?;
    let empty = Project::empty("auto rig");
    let mut document = empty.document();
    let import = import_psd(document.model(), &bytes, &PsdImportOptions::new("auto rig"))?;
    document.apply(import.command)?;

    let labels = classify_by_name(&ugoe_psd::import(&bytes)?);
    let parts = mesh_parts(document.model(), &labels);
    let proposal = propose_rig(
        &document,
        &parts,
        &RigParams::default(),
        &RigOptions::default(),
    )?;
    eprintln!(
        "{} ({} commands)\n{}",
        proposal.title,
        proposal.commands.len(),
        proposal.rationale
    );
    document.apply(Command::Batch {
        label: proposal.title,
        commands: proposal.commands,
    })?;
    let project = Project::from_document(empty.meta, &document);
    std::fs::write(&output, export_bundle(&project)?)?;
    Ok(())
}
