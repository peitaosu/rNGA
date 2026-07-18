# NGA BBS HTTP Interface

Independent notes from the official developer thread, live HTTP probes, and public reverse-engineering docs. Not tied to any client implementation.

**Primary spec:** [NGA数据接口](https://nga.178.com/read.php?tid=6406100) (tid `6406100`, 2013, still the canonical parameter reference).

## Architecture

NGA exposes two overlapping API layers on the same hosts:

| Layer | Entry | Used by |
|-------|-------|---------|
| Web scripts | `thread.php`, `read.php`, `post.php`, `forum.php`, `index.php`, `nuke.php` | Browser, old integrations |
| App router | `app_api.php?__lib=…&__act=…` | Official iOS/Android clients |

Both use POST-heavy, form-encoded requests. Parameters may appear in the URL query, the form body, or both; long payloads (reply text) belong in the form.

## Hosts

| Host | Notes |
|------|-------|
| `ngabbs.com` | Current BBS host |
| `bbs.nga.cn` | Legacy alias, same scripts |
| `nga.178.com` | Portal; login cookies differ until synced |

Cookie bridge (official §10): `nuke.php?__lib=login&__act=set_cookie` accepts `ngaPassportUid`/`ngaPassportCid` or 178 cookies (`id_178c`, `cookie_sid`) and returns redirect URLs per domain.

## Session

Observed cookie chain (live probe, guest):

1. `guestJs=<unix_ts>` — set by client; timestamp cookie
2. `GET /index.php?lite=js` — server assigns `ngaPassportUid=guest<hash>`, `lastvisit`, `lastpath`
3. Logged-in users replace passport with real `ngaPassportUid` + `ngaPassportCid`

Mobile/API requests also send form fields:

```
access_uid=<uid>
access_token=<token>
```

Empty when unauthenticated. Official clients may additionally send `__ngaClientChecksum` (md5 of uid + client secret + timestamp + timestamp) for certified apps (§13).

## Encoding

| Direction | Default | Override |
|-----------|---------|----------|
| Output | GBK / GB18030 | `__output=11` or `14` → UTF-8 JSON |
| Input | GBK | `__inchst=UTF8` in request |

Decode response bytes as GB18030 unless Content-Type indicates UTF-8 JSON.

## Output Formats

Official `lite` / `__output` mapping (§1.1):

| Flag | Shape |
|------|-------|
| `lite=js` | `window.script_muti_get_var_store={data:{…},time:…}` |
| `lite=xml` / `__output=9` | XML under `<root>` |
| `__output=8` | JSON-like object, no JS wrapper, GBK |
| `__output=11` | Standard JSON, UTF-8 |
| `__output=14` | Standard JSON, UTF-8 (current app default) |

Errors in JS mode: `{error:{0:code,1:message,…}}`. XML: `/root/__MESSAGE` or `<error code="…"/>`. App JSON: `{code, msg}` (`code≠0` = failure).

**CDN note (live):** `lite=xml` in the URL query on `app_api.php` returns HTTP 403 with an empty body. Use native JSON on `app_api.php`, or put format flags in the form body on web scripts.

**HTTP status:** Web endpoints often return HTTP 403 while still sending a parseable XML/JS body (e.g. login required). Treat body content as authoritative.

## Identifiers

| Name | Param | Role |
|------|-------|------|
| fid | `fid` | Board ID (`-7` = 网事杂谈) |
| stid | `stid` | Topic collection / sub-board |
| tid | `tid` | Thread |
| pid | `pid` | Post / floor (`0` = OP) |
| mid | `mid` | PM thread |
| uid | `uid`, `authorid` | User |

Board list may list both `fid` and `stid`; collections use `stid` for listing context.

## Web Endpoints (official §3–8)

### `index.php`

`?lite=js` → `{data:{__GLOBAL, index}}` where `index` points at `./template/js/nga_index_forums.xml` (static public board tree).

### `thread.php` — topic list / search

Key query params: `fid`, `stid`, `page`, `authorid`, `key`, `content`, `favor`, `recommend`, `fidgroup=user`, `order_by`.

Response keys: `__T` (topics), `__F` (forum meta), `__ROWS`, `__T__ROWS_PAGE` (35), `__R__ROWS_PAGE` (20).

### `read.php` — thread content

Params: `tid`, `pid`, `page` (`e` = last page), `authorid`, `v2=1`.

Response keys: `__T` (thread), `__R` (posts), `__U` (users), `__F`, `__ROWS`.

### `post.php` — compose pipeline

Two-step flow (§6): (1) `action=reply|quote|new|modify` fetches form + upload auth; (2) `step=2` submits `post_content`, `post_subject`, attachments.

### `forum.php`

`key=<keyword>` — search boards.

### `nuke.php` — backend router

Query `__lib` + `__act`. Documented modules include:

| `__lib` | `__act` | Function |
|---------|---------|----------|
| `ucp` | `get`, `search` | User profile |
| `message` | `list`, `read`, `new`, `reply` | PM (legacy naming) |
| `noti` | `get_list`, `read`, … | Notifications |
| `login` | `account`, `set_cookie`, `iflogin` | Auth |
| `topic_favor_v2` | `add`, `del`, `list_folder` | Topic favorites |
| `forum_favor2` | `forum_favor` | Board favorites (legacy; see below) |
| `post_comment` | `get`, `add` | Comments |
| `topic_recommend` | `add` | Topic vote |

Newer clients also use shortened names (`pm`, `noti`) — act names differ from the 2013 doc; unauthenticated calls may return `{code:1,msg:"ACTION NOT FOUND …"}`.

## `app_api.php` — mobile router

All routes: `POST app_api.php?__lib=<lib>&__act=<act>&…`.

Verified live:

| Route | Result |
|-------|--------|
| `home/category&_v=2` | `{code:0, result:[categories…]}` — full board tree (guest OK) |
| `favorforum/sync` | `{code:0, result:[…]}` when authed; `{code:1,…}` without |
| `favorforum/add`, `favorforum/del` | `{code:0}` with `fid` in form body |

**Board favorites:** Prefer `favorforum/*` on `app_api.php` with `__output=8`. The legacy `nuke.php?__lib=forum_favor2&__act=forum_favor` path with `lite=xml` in the query returns HTTP 403 (empty body) as of 2026.

Catalog from app reverse-engineering (partial):

| `__lib` | `__act` | Purpose |
|---------|---------|---------|
| `home` | `category`, `hasnew`, `recmthreads`, `bannerrecm` | Home feed |
| `subject` | `list`, `search`, `topped`, `hot` | Topics (`fid`, `page`) |
| `post` | `check`, `new`, `reply`, `list` | Posting |
| `read` | (topic read) | Thread detail |
| `user` | `detail`, `subjects`, `replys` | Profiles |
| `message` | `list`, `detail`, `send`, `reply` | PM |
| `notify` | `list`, `unreadcnt` | Notifications |
| `favor` / `favorforum` | various | Favorites |
| `forum` | `search` | Board search |
| `check_in` | `check_in`, `get_stat` | Daily check-in |

App login uses `nuke.php?__lib=login&__act=account` with client sign: `md5(timestamp + appSecret + email + password + app_id)`.

## Pagination

| Context | Rows/page |
|---------|-----------|
| Topic list | 35 (`__T__ROWS_PAGE`) |
| Thread read / comments / PM | 20 (`__R__ROWS_PAGE`) |

`total_pages = ceil(__ROWS / page_size)`.

## Static Assets

Public board XML: `/template/js/nga_index_forums.xml`  
Global constants: `/template/js/nga_global.xml` (`__IMG_BASE`, `__FORUM_ICON_PATH`, …)  
Board icons: `http://img4.ngacn.cc/ngabbs/nga_classic/f/…` (CDN path may vary; app JSON includes `forum_icon_pre`).

## Access Policy (live, 2026)

Probed from `ngabbs.com` without logged-in passport:

| Endpoint | Guest access |
|----------|--------------|
| `app_api.php?home/category` | Yes |
| `index.php`, static XML templates | Yes |
| `thread.php`, `read.php` (web) | No — `{error:…未登录…}` / `__MESSAGE` + HTTP 403 |
| `app_api.php?subject/list` | Empty/`code:5` without auth |
| `nuke.php` authed actions | Requires session |

Guest browsing of threads may now require valid passport cookies plus mobile client headers, or full login. Behavior can vary by board permissions (`view_privilege` link appears in denial messages).

## rNGA mapping

Endpoint and format choices for this repo: [CLIENT.md](./CLIENT.md).

## References

- [Official API thread (tid 6406100)](https://nga.178.com/read.php?tid=6406100)
- [AgMonk/nga-api-doc](https://github.com/AgMonk/nga-api-doc) — HTTP/format notes
- [wolfcon/NGA-API-Documents](https://github.com/wolfcon/NGA-API-Documents) — `app_api.php` route list
