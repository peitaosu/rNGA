# rNGA Client HTTP Routing

How the `rnga` library chooses endpoints and response formats. For NGA protocol details see [NGA.md](./NGA.md).

## Policy

One route per feature — no fallback chains or format retries.

| Transport | Format flag | Used for |
|-----------|-------------|----------|
| `post()` / `post_authed()` | `lite=xml` (WebXml) | `thread.php`, `read.php`, `forum.php`, `nuke.php` |
| `post_json()` / `post_json_authed()` | `__output=8` (AppJson) | `app_api.php` |

Do not put `lite=xml` in the query on `app_api.php` — CDN returns HTTP 403 with an empty body.

Auth uses form fields `access_uid` + `access_token` (from `ngaPassportUid` / `ngaPassportCid`).

## Route map

| Feature | Script | Lib / act or params | Format |
|---------|--------|---------------------|--------|
| Forum categories | `app_api.php` | `home` / `category`, `_v=2` | JSON |
| Forum search | `forum.php` | `key` | XML |
| Forum favorites list | `app_api.php` | `favorforum` / `sync` | JSON |
| Forum favorites add/remove | `app_api.php` | `favorforum` / `add` or `del`, `fid` | JSON |
| Topic list | `thread.php` | `fid` or `stid`, `page`, `order_by` | XML |
| Topic read | `read.php` | `tid`, `page`, `v2=1` | XML |
| Topic search | `thread.php` | `fid`/`stid`, `key`, filters | XML |
| Topic favorites | `thread.php` | `favor`, `page` | XML |
| Topic favorite folders / add / del | `nuke.php` | `topic_favor_v2` | XML |
| User topics | `thread.php` | `authorid`, `page` | XML |
| User profile / search | `nuke.php` | `ucp` | XML |
| Post reply / vote / comments | `nuke.php` / `post.php` | varies | XML |
| Notifications | `nuke.php` | `noti` | XML |
| Messages | `nuke.php` | `message` | XML |
| Subforum filter | `nuke.php` | `user_option` / `set` | XML |

## Surfaces

The same library backs CLI, TUI, and MCP. Auth and config live in `~/.config/rnga/config.toml` (override with `RNGA_CONFIG_DIR` or `XDG_CONFIG_HOME`).

| Surface | Entry |
|---------|-------|
| CLI | `rnga …` |
| TUI | `rnga tui [--forum ID] [--stid] [--topic TID]` |
| MCP | `rnga --mcp` |

CLI command reference: [rNGA-CLI/README.md](../rNGA-CLI/README.md).
