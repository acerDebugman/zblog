# zblog

一个自托管的个人博客系统：Rust 后端 + Astro SSR 前端，管理后台支持文章编辑与访问统计。

- **后端**：Rust（axum + sqlx + SQLite），位于 `server/`
- **前端**：Astro（SSR，standalone Node 适配器）+ Vue 3 组件，位于 `web/`
- **架构决策记录**：`docs/adr/`

## 部署拓扑

遵循 ADR-0003：**Astro（Node）进程是唯一的公网入口**，Rust API 只监听 `127.0.0.1:8080`。

- 浏览器所有流量（公开页面、管理页面、API 调用）都经由 Node 进程；`/api/admin/*` 请求由 Astro 的同源代理转发到 Rust 进程。
- 生产部署为同一台主机上的两个进程：Astro 面向公网，通过 localhost 调用 Rust API；不引入 nginx 等额外组件。
- 管理端认证（密码 + 签名会话 Cookie）在 API 层强制执行，"仅内网监听" 只是部署姿态，不是安全边界。

## 环境变量

### 后端（server/）

| 变量 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `ZBLOG_PASSWORD_HASH` | 是 | 无 | 管理员密码的 Argon2 PHC 哈希。生成方式：`cargo run --example hash_password -- '<密码>'`（在 `server/` 下执行） |
| `ZBLOG_SESSION_SECRET` | 是 | 无 | 会话 Cookie 签名密钥，使用至少 64 个随机字符 |
| `ZBLOG_BIND_ADDR` | 否 | `127.0.0.1:8080` | HTTP 服务监听地址，保持默认即可 |
| `ZBLOG_DATABASE_URL` | 否 | `sqlite:zblog.db` | SQLite 连接 URL |

### 前端（web/）

| 变量 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `RUST_API_URL` | 否 | `http://127.0.0.1:8080` | SSR 侧调用 Rust API 的地址，仅在非本机部署时需要修改 |

## 构建与运行

### 后端

```sh
cd server
ZBLOG_PASSWORD_HASH='...' ZBLOG_SESSION_SECRET='...' cargo run
```

数据库迁移在启动时自动执行（`server/migrations/`）。

### 前端

```sh
cd web
pnpm install
pnpm build
node ./dist/server/entry.mjs
```

开发模式（Astro dev server 会把 `/api` 代理到 Rust 进程）：

```sh
cd web && pnpm dev
```

## 测试

```sh
# 后端
cargo test --manifest-path server/Cargo.toml

# 前端单元测试
cd web && pnpm vitest run

# 前端类型检查
cd web && pnpm astro check
```

## 安全提示

登录接口目前没有限流（throttling），请务必使用高强度管理员密码。
