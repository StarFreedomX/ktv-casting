# 与[ktv-song-web](https://github.com/StarFreedomX/ktv-song-web)搭配的命令行DLNA投屏软件

[开发者文档](docs/DEVELOPER.md)
## 使用方式

输入搭建好的[ktv-song-web](https://github.com/StarFreedomX/ktv-song-web)服务的网址（含对应房间编号），如`http://ktv.example.com/101`，随后选择搜索到的DLNA设备，即可使用。

## 功能

跟随网页的正在播放曲目进行投屏，结束自动切歌。也可以在网页端操作进行切歌。

## 技术更新

### WebSocket实时通信（推荐）

从ktv-song-web v0.4.0开始，项目已实现WebSocket功能。本客户端优先使用WebSocket连接，实现实时歌单更新推送，无需轮询。

**优势：**
- 实时性更高：服务器主动推送歌单变化
- 资源消耗更低：无需频繁HTTP轮询
- 连接更稳定：支持心跳检测和自动重连

### 轮询备用方案

如果WebSocket连接失败（例如服务器版本较旧或网络限制），客户端会自动退回到HTTP轮询模式，确保兼容性。

## 手机上怎么用

1. 下载并安装[Termux](https://termux.com/)。
2. 从[这里](https://github.com/aspromise/ktv-casting/releases)下载最新的`ktv-casting-aarch64-linux-android`可执行文件。
建议可以直接在Termux中使用`curl -LO <下载链接>`命令下载。以`v0.1.5`版本为例，命令如下：
```bash
curl -LO https://github.com/aspromise/ktv-casting/releases/download/v0.1.5/ktv-casting-aarch64-linux-android
```
3. 赋予可执行权限：
```bash
chmod +x ktv-casting-aarch64-linux-android
```
4. 运行程序：
```bash
./ktv-casting-aarch64-linux-android
```

