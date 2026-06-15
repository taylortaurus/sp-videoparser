#!/usr/bin/env node
// 将指定版本号写入主包与全部平台子包的 package.json，
// 并同步主包 optionalDependencies 中各子包的版本，确保一致。
//
// 用法: node npm/scripts/prepare.mjs <version>
//   <version> 可带或不带前缀 v，如 2.1.0 或 v2.1.0

import { readFileSync, writeFileSync, readdirSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const npmDir = join(dirname(fileURLToPath(import.meta.url)), "..");

const raw = process.argv[2];
if (!raw) {
  console.error("用法: node npm/scripts/prepare.mjs <version>");
  process.exit(1);
}
const version = raw.replace(/^v/, "");
if (!/^\d+\.\d+\.\d+/.test(version)) {
  console.error(`非法版本号: ${raw}`);
  process.exit(1);
}

function patch(pkgPath, mutate) {
  const json = JSON.parse(readFileSync(pkgPath, "utf8"));
  mutate(json);
  writeFileSync(pkgPath, JSON.stringify(json, null, 2) + "\n");
  console.log(`已更新 ${pkgPath} -> ${json.version}`);
}

// 主包
const mainPkg = join(npmDir, "rust-video-parser", "package.json");
patch(mainPkg, (json) => {
  json.version = version;
  for (const dep of Object.keys(json.optionalDependencies || {})) {
    json.optionalDependencies[dep] = version;
  }
});

// 平台子包（@scope 目录形如 win32-x64、linux-arm64 等）
for (const entry of readdirSync(npmDir, { withFileTypes: true })) {
  if (!entry.isDirectory() || entry.name === "rust-video-parser" || entry.name === "scripts") {
    continue;
  }
  const sub = join(npmDir, entry.name, "package.json");
  if (existsSync(sub)) {
    patch(sub, (json) => {
      json.version = version;
    });
  }
}

console.log(`全部包版本已同步为 ${version}`);
