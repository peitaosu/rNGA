# rNGA 客户端 HTTP 路由

说明 `rnga` 库如何选择端点与响应格式。NGA 协议细节见 [NGA.md](./NGA.md)。

## 策略

每个功能只走一条路由 — 无回退链、无格式重试。

| 传输 | 格式标志 | 用于 |
|------|----------|------|
| `post()` / `post_authed()` | `lite=xml`（WebXml） | `thread.php`、`read.php`、`forum.php`、`nuke.php` |
| `post_json()` / `post_json_authed()` | `__output=8`（AppJson） | `app_api.php` |

不要在 `app_api.php` 的查询串中使用 `lite=xml` — CDN 会返回 HTTP 403 且 body 为空。

认证使用表单字段 `access_uid` + `access_token`（来自 `ngaPassportUid` / `ngaPassportCid`）。

## 路由表

| 功能 | 脚本 | lib / act 或参数 | 格式 |
|------|------|------------------|------|
| 版块分类 | `app_api.php` | `home` / `category`，`_v=2` | JSON |
| 版块搜索 | `forum.php` | `key` | XML |
| 版块收藏列表 | `app_api.php` | `favorforum` / `sync` | JSON |
| 版块收藏添加/删除 | `app_api.php` | `favorforum` / `add` 或 `del`，`fid` | JSON |
| 主题列表 | `thread.php` | `fid` 或 `stid`、`page`、`order_by` | XML |
| 主题阅读 | `read.php` | `tid`、`page`、`v2=1` | XML |
| 主题搜索 | `thread.php` | `fid`/`stid`、`key`、过滤器 | XML |
| 主题收藏 | `thread.php` | `favor`、`page` | XML |
| 主题收藏夹 / 添加 / 删除 | `nuke.php` | `topic_favor_v2` | XML |
| 用户主题 | `thread.php` | `authorid`、`page` | XML |
| 用户资料 / 搜索 | `nuke.php` | `ucp` | XML |
| 回帖 / 投票 / 评论 | `nuke.php` / `post.php` | 各异 | XML |
| 通知 | `nuke.php` | `noti` | XML |
| 私信 | `nuke.php` | `message` | XML |
| 子版块过滤 | `nuke.php` | `user_option` / `set` | XML |

## 使用面

同一库支撑 CLI、TUI 与 MCP。认证与配置位于 `~/.config/rnga/config.toml`（可通过 `RNGA_CONFIG_DIR` 或 `XDG_CONFIG_HOME` 覆盖）。

| 使用面 | 入口 |
|--------|------|
| CLI | `rnga …` |
| TUI | `rnga` 或 `rnga tui [--forum ID] [--stid] [--topic TID]` |
| MCP | `rnga --mcp` |

CLI 命令参考：[rNGA-CLI/README.md](../../rNGA-CLI/README.md)。
