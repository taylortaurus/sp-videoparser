# rust-video-parser

企业级全平台短视频解析 CLI。基于 Rust 实现，26 个平台无水印解析，通过 npm 分发预编译多平台二进制。

## 安装

```bash
npm install -g rust-video-parser
```

安装时 npm 会依据当前系统自动选取匹配的预编译二进制（`@rust-video-parser/<platform>-<arch>`），无需本地工具链。

## 使用

```bash
# 解析分享链接
rust-video parse "https://v.douyin.com/xxxxxx/"

# 机器可读 JSON（结构同 HTTP API）
rust-video parse "https://v.douyin.com/xxxxxx/" --json

# 按平台与视频 ID 解析
rust-video id 7234567890123456789 --source douyin

# 下载视频/图集/封面/音乐
rust-video download "https://v.douyin.com/xxxxxx/" --output ./media

# 列出受支持平台
rust-video platforms

# 启动 HTTP 服务（前端已内嵌，开箱即用）
rust-video serve --port 8080
```

## 支持平台

win32-x64、darwin-x64、darwin-arm64、linux-x64、linux-arm64。

其它平台请使用 Docker 镜像或源码构建，详见
[项目主页](https://github.com/eginner01/rust_video)。

## 许可证

MIT
