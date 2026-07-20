# NGA 论坛 HTTP 接口

基于官方开发者帖、线上 HTTP 探测与公开逆向文档整理的独立说明，不绑定任何具体客户端实现。

**主要规范：** [NGA数据接口](https://nga.178.com/read.php?tid=6406100)（tid `6406100`，2013 年发布，至今仍是参数参考的权威来源）。

## 架构

NGA 在同一组主机上暴露两套重叠的 API 层：

| 层级 | 入口 | 使用者 |
|------|------|--------|
| Web 脚本 | `thread.php`、`read.php`、`post.php`、`forum.php`、`index.php`、`nuke.php` | 浏览器、旧版集成 |
| App 路由 | `app_api.php?__lib=…&__act=…` | 官方 iOS/Android 客户端 |

两者均以 POST 为主、表单编码请求。参数可出现在 URL 查询串、表单 body，或两者兼有；较长载荷（如回复正文）应放在表单 body 中。

## 主机

| 主机 | 说明 |
|------|------|
| `ngabbs.com` | 当前 BBS 主机 |
| `bbs.nga.cn` | 旧版别名，脚本相同 |
| `nga.178.com` | 门户站；登录 Cookie 在同步前与 BBS 不同 |

Cookie 桥接（官方 §10）：`nuke.php?__lib=login&__act=set_cookie` 接受 `ngaPassportUid`/`ngaPassportCid` 或 178 Cookie（`id_178c`、`cookie_sid`），并按域名返回重定向 URL。

## 会话

观测到的 Cookie 链（线上探测，访客）：

1. `guestJs=<unix_ts>` — 由客户端设置；时间戳 Cookie
2. `GET /index.php?lite=js` — 服务端分配 `ngaPassportUid=guest<hash>`、`lastvisit`、`lastpath`
3. 登录用户以真实的 `ngaPassportUid` + `ngaPassportCid` 替换 passport

移动端/API 请求还会发送表单字段：

```
access_uid=<uid>
access_token=<token>
```

未认证时为空。官方客户端可能额外发送 `__ngaClientChecksum`（uid + 客户端密钥 + 时间戳 + 时间戳 的 md5），用于认证应用（§13）。

## 编码

| 方向 | 默认 | 覆盖方式 |
|------|------|----------|
| 输出 | GBK / GB18030 | `__output=11` 或 `14` → UTF-8 JSON |
| 输入 | GBK | 请求中带 `__inchst=UTF8` |

除非 Content-Type 表明为 UTF-8 JSON，否则响应字节应按 GB18030 解码。

## 输出格式

官方 `lite` / `__output` 映射（§1.1）：

| 标志 | 形态 |
|------|------|
| `lite=js` | `window.script_muti_get_var_store={data:{…},time:…}` |
| `lite=xml` / `__output=9` | `<root>` 下的 XML |
| `__output=8` | 类 JSON 对象，无 JS 包装，GBK |
| `__output=11` | 标准 JSON，UTF-8 |
| `__output=14` | 标准 JSON，UTF-8（当前 App 默认） |

JS 模式错误：`{error:{0:code,1:message,…}}`。XML：`/root/__MESSAGE` 或 `<error code="…"/>`。App JSON：`{code, msg}`（`code≠0` 表示失败）。

**CDN 说明（实测）：** 在 `app_api.php` 的 URL 查询串中使用 `lite=xml` 会返回 HTTP 403 且 body 为空。`app_api.php` 应使用原生 JSON；Web 脚本可将格式标志放在表单 body 中。

**HTTP 状态码：** Web 端点常返回 HTTP 403，但 body 仍可解析（如需要登录）。应以 body 内容为准。

## 标识符

| 名称 | 参数 | 作用 |
|------|------|------|
| fid | `fid` | 版块 ID（`-7` = 网事杂谈） |
| stid | `stid` | 主题合集 / 子版块 |
| tid | `tid` | 主题帖 |
| pid | `pid` | 楼层 / 回帖（`0` = 主楼） |
| mid | `mid` | 私信会话 |
| uid | `uid`、`authorid` | 用户 |

版块列表可能同时包含 `fid` 与 `stid`；合集在列表上下文中使用 `stid`。

## Web 端点（官方 §3–8）

### `index.php`

`?lite=js` → `{data:{__GLOBAL, index}}`，其中 `index` 指向 `./template/js/nga_index_forums.xml`（静态公开版块树）。

### `thread.php` — 主题列表 / 搜索

主要查询参数：`fid`、`stid`、`page`、`authorid`、`key`、`content`、`favor`、`recommend`、`fidgroup=user`、`order_by`。

响应键：`__T`（主题）、`__F`（版块元数据）、`__ROWS`、`__T__ROWS_PAGE`（35）、`__R__ROWS_PAGE`（20）。

### `read.php` — 主题内容

参数：`tid`、`pid`、`page`（`e` = 末页）、`authorid`、`v2=1`。

响应键：`__T`（主题）、`__R`（帖子）、`__U`（用户）、`__F`、`__ROWS`。

### `post.php` — 发帖流程

两步流程（§6）：(1) `action=reply|quote|new|modify` 获取表单与上传授权；(2) `step=2` 提交 `post_content`、`post_subject`、附件。

### `forum.php`

`key=<keyword>` — 搜索版块。

### `nuke.php` — 后端路由

查询参数 `__lib` + `__act`。文档化模块包括：

| `__lib` | `__act` | 功能 |
|---------|---------|------|
| `ucp` | `get`、`search` | 用户资料 |
| `message` | `list`、`read`、`new`、`reply` | 私信（旧命名） |
| `noti` | `get_list`、`read`、… | 通知 |
| `login` | `account`、`set_cookie`、`iflogin` | 认证 |
| `topic_favor_v2` | `add`、`del`、`list_folder` | 主题收藏 |
| `forum_favor2` | `forum_favor` | 版块收藏（旧版；见下文） |
| `post_comment` | `get`、`add` | 评论 |
| `topic_recommend` | `add` | 主题投票 |

较新客户端还使用缩短名称（`pm`、`noti`）——`__act` 名称与 2013 文档不同；未认证调用可能返回 `{code:1,msg:"ACTION NOT FOUND …"}`。

## `app_api.php` — 移动端路由

所有路由：`POST app_api.php?__lib=<lib>&__act=<act>&…`。

已验证可用：

| 路由 | 结果 |
|------|------|
| `home/category&_v=2` | `{code:0, result:[categories…]}` — 完整版块树（访客可用） |
| `favorforum/sync` | 已认证时 `{code:0, result:[…]}`；未认证 `{code:1,…}` |
| `favorforum/add`、`favorforum/del` | 表单 body 带 `fid` 时 `{code:0}` |

**版块收藏：** 优先在 `app_api.php` 上使用 `favorforum/*` 并设 `__output=8`。旧路径 `nuke.php?__lib=forum_favor2&__act=forum_favor` 在查询串中带 `lite=xml` 截至 2026 年返回 HTTP 403（body 为空）。

App 逆向目录（部分）：

| `__lib` | `__act` | 用途 |
|---------|---------|------|
| `home` | `category`、`hasnew`、`recmthreads`、`bannerrecm` | 首页 |
| `subject` | `list`、`search`、`topped`、`hot` | 主题（`fid`、`page`） |
| `post` | `check`、`new`、`reply`、`list` | 发帖 |
| `read` | （主题阅读） | 主题详情 |
| `user` | `detail`、`subjects`、`replys` | 用户资料 |
| `message` | `list`、`detail`、`send`、`reply` | 私信 |
| `notify` | `list`、`unreadcnt` | 通知 |
| `favor` / `favorforum` | 多种 | 收藏 |
| `forum` | `search` | 版块搜索 |
| `check_in` | `check_in`、`get_stat` | 每日签到 |

App 登录使用 `nuke.php?__lib=login&__act=account`，客户端签名：`md5(timestamp + appSecret + email + password + app_id)`。

## 分页

| 场景 | 每页行数 |
|------|----------|
| 主题列表 | 35（`__T__ROWS_PAGE`） |
| 主题阅读 / 评论 / 私信 | 20（`__R__ROWS_PAGE`） |

`total_pages = ceil(__ROWS / page_size)`。

## 静态资源

公开版块 XML：`/template/js/nga_index_forums.xml`  
全局常量：`/template/js/nga_global.xml`（`__IMG_BASE`、`__FORUM_ICON_PATH` 等）  
版块图标：`http://img4.ngacn.cc/ngabbs/nga_classic/f/…`（CDN 路径可能变化；App JSON 含 `forum_icon_pre`）。

## 访问策略（实测，2026）

从 `ngabbs.com` 无登录 passport 探测：

| 端点 | 访客访问 |
|------|----------|
| `app_api.php?home/category` | 可以 |
| `index.php`、静态 XML 模板 | 可以 |
| `thread.php`、`read.php`（Web） | 不可以 — `{error:…未登录…}` / `__MESSAGE` + HTTP 403 |
| `app_api.php?subject/list` | 未认证时为空 / `code:5` |
| `nuke.php` 需认证操作 | 需要会话 |

访客浏览主题帖现在可能需要有效 passport Cookie 加移动端客户端头，或完整登录。行为因版块权限（`view_privilege` 链接会出现在拒绝消息中）而异。

## rNGA 映射

本仓库的端点与格式选择见 [CLIENT.md](./CLIENT.md)。

## 参考资料

- [官方 API 帖（tid 6406100）](https://nga.178.com/read.php?tid=6406100)
- [AgMonk/nga-api-doc](https://github.com/AgMonk/nga-api-doc) — HTTP/格式说明
- [wolfcon/NGA-API-Documents](https://github.com/wolfcon/NGA-API-Documents) — `app_api.php` 路由列表
