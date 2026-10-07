//! fuzz のための安全な入口 (`ugoe_ffi::fuzzing`。feature `fuzzing`。T0497) を、決まった入力で stable で回す。
//! fuzz のターゲット (`fuzz/fuzz_targets/ffi.rs`) の種と同じファイルに、すべての操作を含む台本を付けて、
//! C ABI の中でパニックせず、返した値がホストへの約束を守ることを確かめる。fuzz が見つけた入力もここに足す (docs/testing.md#fuzz)。
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};

use ugoe_ffi::fuzzing::{SCRIPT_MARKER, run};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn dress_up_base() -> Vec<u8> {
    fs::read(root().join("assets/compat/dev/dress-up-base.ugoe")).unwrap()
}

/// `assets/compat` と `assets/samples` のバンドルとモジュール。
fn files() -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    for dir in [
        "assets/compat/dev",
        "assets/compat/future",
        "assets/samples",
        "assets/samples/dress-up",
    ] {
        for entry in fs::read_dir(root().join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("ugoe" | "ugoemod")
            ) {
                let bytes = fs::read(&path).unwrap();
                files.push((path, bytes));
            }
        }
    }
    assert!(files.len() > 20, "{} files", files.len());
    files
}

/// 決まった台本: 操作ごとに、UgoeError あり・なしと、続くバイトを変えて並べる。
fn scripts() -> Vec<Vec<u8>> {
    let mut scripts = vec![Vec::new()];
    // すべての操作 (0〜23) を、続くバイトを 0 (範囲の中・数の表の先頭) にして
    scripts.push((0..24u8).flat_map(|op| [op, 0, 0, 0, 0, 0]).collect());
    // UgoeError を渡さず、続くバイトを端の値にして (範囲の外の番号・NaN・無限大・長い名前)
    for fill in [0x06, 0x07, 0x08, 0x40, 0xBF, 0xC3, 0xF0, 0xF1, 0xFF] {
        scripts.push(
            (0..24u8)
                .flat_map(|op| [op, 0x80, fill, fill, fill, fill, fill, fill])
                .collect(),
        );
    }
    // 疑似乱数 (決まった種の線形合同法) の台本
    let mut state = 0x2545_f491_u32;
    for _ in 0..4 {
        let script = (0..400)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 24) as u8
            })
            .collect();
        scripts.push(script);
    }
    scripts
}

fn input(file: &[u8], script: &[u8]) -> Vec<u8> {
    let mut input = file.to_vec();
    input.extend_from_slice(SCRIPT_MARKER);
    input.extend_from_slice(script);
    input
}

#[test]
fn seeds_with_scripts_do_not_panic_inside_the_c_abi() {
    let base = dress_up_base();
    let scripts = scripts();
    for (path, file) in files() {
        for (i, script) in scripts.iter().enumerate() {
            if let Err(failure) = run(&input(&file, script), &base) {
                panic!("{} with script {i}: {failure}", path.display());
            }
        }
    }
}

#[test]
fn broken_inputs_do_not_panic_inside_the_c_abi() {
    let base = dress_up_base();
    let scripts = scripts();
    let script = &scripts[1];
    for input_bytes in [
        Vec::new(),
        SCRIPT_MARKER.to_vec(),
        b"{}".to_vec(),
        b"{\"schema_version\":1}".to_vec(),
        vec![0xff; 64],
    ] {
        run(&input_bytes, &base).unwrap();
        run(&input(&input_bytes, script), &base).unwrap();
    }
    // 切り詰めたバンドルとモジュール
    for (path, file) in files().into_iter().take(6) {
        for len in [0, 8, 16, 64, file.len() / 2, file.len() - 1] {
            let cut = file.get(..len).unwrap();
            if let Err(failure) = run(&input(cut, script), &base) {
                panic!("{} cut at {len}: {failure}", path.display());
            }
        }
    }
}
