// scripts/ffi-dist.mjs (C ABI の配布物) の単体テスト (`just script-check` が実行する)。
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";
import { PLUGINS } from "../bindings/unity/build.mjs";
import {
  abiVersionFromHeader,
  abiVersionFromRust,
  archiveName,
  BUILDS,
  builtFiles,
  cargoCommands,
  expectedFiles,
  PLATFORMS,
  parseArchiveName,
  platformOf,
  readelfNames,
  sharedLibraryId,
  tagProblem,
} from "./ffi-dist.mjs";

const crate = join(import.meta.dirname, "..", "crates", "ugoe-ffi");

describe("platformOf", () => {
  it("maps the hosts that have an archive", () => {
    assert.equal(platformOf("linux", "x64"), "linux-x86_64");
    assert.equal(platformOf("linux", "arm64"), "linux-aarch64");
    assert.equal(platformOf("win32", "x64"), "windows-x86_64");
    assert.equal(platformOf("darwin", "arm64"), "macos-universal");
    assert.equal(platformOf("darwin", "x64"), "macos-universal");
  });

  it("returns null for hosts without an archive", () => {
    assert.equal(platformOf("win32", "arm64"), null);
    assert.equal(platformOf("freebsd", "x64"), null);
  });
});

describe("archiveName", () => {
  it("uses zip on Windows and tar.gz elsewhere", () => {
    assert.equal(archiveName("0.1.0", "windows-x86_64"), "ugoe-ffi-0.1.0-windows-x86_64.zip");
    assert.equal(archiveName("0.1.0", "linux-aarch64"), "ugoe-ffi-0.1.0-linux-aarch64.tar.gz");
    assert.equal(archiveName("0.1.0", "macos-universal"), "ugoe-ffi-0.1.0-macos-universal.tar.gz");
  });

  it("round-trips through parseArchiveName for every platform", () => {
    for (const platform of Object.keys(PLATFORMS)) {
      assert.deepEqual(parseArchiveName(archiveName("1.2.3-rc.1", platform)), {
        version: "1.2.3-rc.1",
        platform,
      });
    }
  });

  it("rejects other names and mismatched formats", () => {
    assert.equal(parseArchiveName("ugoe-ffi-0.1.0-linux-x86_64.zip"), null);
    assert.equal(parseArchiveName("ugoe-ffi-0.1.0-windows-x86_64.tar.gz"), null);
    assert.equal(parseArchiveName("ugoe-ffi-0.1.0-linux-riscv64.tar.gz"), null);
    assert.equal(parseArchiveName("com.ugoe.unity-0.1.0.tgz"), null);
  });
});

describe("expectedFiles", () => {
  it("lists the header, the libraries, the sample, LICENSE, README and the notices", () => {
    assert.deepEqual(expectedFiles("windows-x86_64", 3), [
      "LICENSE",
      "README.md",
      "THIRD-PARTY-NOTICES.txt",
      "examples/sample.c",
      "include/ugoe.h",
      "lib/ugoe_ffi.dll",
      "lib/ugoe_ffi.dll.lib",
      "lib/ugoe_ffi.lib",
    ]);
    assert.deepEqual(expectedFiles("linux-x86_64", 3), [
      "LICENSE",
      "README.md",
      "THIRD-PARTY-NOTICES.txt",
      "examples/sample.c",
      "include/ugoe.h",
      "lib/libugoe_ffi.a",
      "lib/libugoe_ffi.so",
      "lib/libugoe_ffi.so.3",
    ]);
    assert.deepEqual(
      expectedFiles("macos-universal", 4).filter((f) => f.startsWith("lib/")),
      ["lib/libugoe_ffi.4.dylib", "lib/libugoe_ffi.a", "lib/libugoe_ffi.dylib"],
    );
  });

  it("ships a shared and a static library on every platform", () => {
    for (const [platform, { lib }] of Object.entries(PLATFORMS)) {
      assert.ok(
        lib.some((f) => /\.(so|dll|dylib)$/.test(f)),
        platform,
      );
      assert.ok(
        lib.some((f) => /\.a$|^ugoe_ffi\.lib$/.test(f)),
        platform,
      );
    }
  });
});

describe("tagProblem", () => {
  it("accepts v<version> only", () => {
    assert.equal(tagProblem("v0.1.0", "0.1.0"), null);
    assert.match(tagProblem("v0.1.1", "0.1.0"), /does not match/);
    assert.match(tagProblem("0.1.0", "0.1.0"), /expected v0\.1\.0/);
  });
});

describe("ABI version", () => {
  it("reads the version from lib.rs and the header", () => {
    assert.equal(abiVersionFromRust("/// doc\npub const ABI_VERSION: u32 = 12;\n"), 12);
    assert.equal(abiVersionFromRust("const ABI_VERSION: u32 = 1;"), null);
    assert.equal(abiVersionFromHeader("#define UGOE_ABI_VERSION 7\n"), 7);
    assert.equal(abiVersionFromHeader("// UGOE_ABI_VERSION 7\n"), null);
  });

  it("is the same in lib.rs and the committed header", () => {
    const rust = abiVersionFromRust(readFileSync(join(crate, "src", "lib.rs"), "utf8"));
    const header = abiVersionFromHeader(readFileSync(join(crate, "include", "ugoe.h"), "utf8"));
    assert.ok(Number.isInteger(rust));
    assert.equal(header, rust);
  });

  it("names the shared library after the ABI version on Linux and macOS only", () => {
    assert.equal(sharedLibraryId("linux-x86_64", 3), "libugoe_ffi.so.3");
    assert.equal(sharedLibraryId("linux-aarch64", 3), "libugoe_ffi.so.3");
    assert.equal(sharedLibraryId("macos-universal", 3), "@rpath/libugoe_ffi.3.dylib");
    assert.equal(sharedLibraryId("windows-x86_64", 3), null);
  });
});

describe("readelfNames", () => {
  it("picks the names of a dynamic section tag", () => {
    const output = [
      "Dynamic section at offset 0x1 contains 3 entries:",
      "  Tag        Type                         Name/Value",
      " 0x0000000000000001 (NEEDED)             Shared library: [libugoe_ffi.so.3]",
      " 0x0000000000000001 (NEEDED)             Shared library: [libc.so.6]",
      " 0x000000000000000e (SONAME)             Library soname: [libugoe_ffi.so.3]",
    ].join("\n");
    assert.deepEqual(readelfNames(output, "NEEDED"), ["libugoe_ffi.so.3", "libc.so.6"]);
    assert.deepEqual(readelfNames(output, "SONAME"), ["libugoe_ffi.so.3"]);
    assert.deepEqual(readelfNames("", "SONAME"), []);
  });
});

describe("BUILDS", () => {
  it("builds every C ABI archive platform", () => {
    for (const platform of Object.keys(PLATFORMS)) assert.ok(BUILDS[platform], platform);
  });

  it("has a plugin for every Unity plugin from bindings/unity/build.mjs and nothing else", () => {
    const plugins = Object.entries(BUILDS)
      .filter(([, { plugin }]) => plugin)
      .map(([platform, { plugin }]) => `${platform}/${plugin}`)
      .sort();
    assert.deepEqual(plugins, PLUGINS.map((p) => p.from).sort());
  });

  it("names the Unity plugins without the ABI version", () => {
    for (const { plugin } of Object.values(BUILDS)) {
      if (plugin) assert.match(plugin, /^(lib)?ugoe_ffi\.(so|dll|dylib|a)$/);
    }
  });

  it("puts the built files under target/<triple>/release", () => {
    assert.deepEqual(builtFiles("macos-universal", "libugoe_ffi.a", "t"), [
      join("t", "aarch64-apple-darwin", "release", "libugoe_ffi.a"),
      join("t", "x86_64-apple-darwin", "release", "libugoe_ffi.a"),
    ]);
    assert.deepEqual(builtFiles("windows-x86_64", "ugoe_ffi.dll", "t"), [
      join("t", "x86_64-pc-windows-msvc", "release", "ugoe_ffi.dll"),
    ]);
  });
});

describe("cargoCommands", () => {
  it("builds each target with --target and --locked", () => {
    assert.deepEqual(cargoCommands("linux-x86_64", { hostOs: "linux" }), [
      [
        "cargo",
        [
          "build",
          "--locked",
          "--release",
          "-p",
          "ugoe-ffi",
          "--target",
          "x86_64-unknown-linux-gnu",
        ],
      ],
    ]);
    assert.deepEqual(
      cargoCommands("macos-universal", { hostOs: "darwin" }).map(([, args]) => args.at(-1)),
      ["aarch64-apple-darwin", "x86_64-apple-darwin"],
    );
    assert.deepEqual(cargoCommands("ios-arm64", { hostOs: "darwin" })[0][1].slice(-2), [
      "--target",
      "aarch64-apple-ios",
    ]);
  });

  it("passes the soname to the linker only when asked (the C ABI archive)", () => {
    const [[, args]] = cargoCommands("linux-x86_64", {
      soname: "libugoe_ffi.so.3",
      hostOs: "linux",
    });
    assert.equal(args[0], "rustc");
    assert.ok(args.includes("--lib"));
    assert.deepEqual(args.slice(-3), ["--", "-C", "link-arg=-Wl,-soname,libugoe_ffi.so.3"]);
    for (const platform of Object.keys(BUILDS)) {
      for (const [, a] of cargoCommands(platform, { hostOs: "linux" })) {
        assert.ok(!a.some((x) => x.includes("soname")), platform);
      }
    }
  });

  it("uses cargo-ndk with API level 21 for Android", () => {
    assert.deepEqual(cargoCommands("android-armeabi-v7a", { hostOs: "linux" }), [
      [
        "cargo",
        [
          "ndk",
          "--target",
          "armeabi-v7a",
          "--platform",
          "21",
          "build",
          "--locked",
          "--release",
          "-p",
          "ugoe-ffi",
        ],
      ],
    ]);
  });

  it("uses cargo-xwin for Windows outside Windows", () => {
    assert.equal(cargoCommands("windows-x86_64", { hostOs: "win32" })[0][1][0], "build");
    assert.deepEqual(cargoCommands("windows-x86_64", { hostOs: "linux" })[0][1].slice(0, 2), [
      "xwin",
      "build",
    ]);
  });
});
