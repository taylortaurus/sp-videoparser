#!/usr/bin/env node
"use strict";

// 启动壳：根据当前平台/架构定位对应的预编译二进制并执行。
// 各平台二进制由 @rust-video-parser/<platform>-<arch> 可选依赖提供，
// npm 仅会安装与当前系统匹配的那一个（依据其 os/cpu 字段）。

const { spawnSync } = require("child_process");

const PLATFORM = process.platform; // win32 | darwin | linux
const ARCH = process.arch; // x64 | arm64

const SUPPORTED = new Set([
  "win32-x64",
  "darwin-x64",
  "darwin-arm64",
  "linux-x64",
  "linux-arm64",
]);

const key = `${PLATFORM}-${ARCH}`;
const pkg = `@rust-video-parser/${key}`;
const exe = PLATFORM === "win32" ? "rust_video_parser.exe" : "rust_video_parser";

function fail(msg) {
  process.stderr.write(`rust-video-parser: ${msg}\n`);
  process.exit(1);
}

if (!SUPPORTED.has(key)) {
  fail(
    `暂不支持的平台: ${key}。\n` +
      `支持的平台: ${[...SUPPORTED].join(", ")}。\n` +
      `可改用 Docker 镜像或源码构建: https://github.com/eginner01/rust_video`
  );
}

let binPath;
try {
  binPath = require.resolve(`${pkg}/bin/${exe}`);
} catch (e) {
  fail(
    `未找到平台二进制包 ${pkg}。\n` +
      `请确认安装时未禁用可选依赖（不要使用 --no-optional / --ignore-optional），\n` +
      `然后重新安装: npm install -g rust-video-parser`
  );
}

const result = spawnSync(binPath, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  fail(`无法执行二进制: ${result.error.message}`);
}
process.exit(result.status === null ? 1 : result.status);
