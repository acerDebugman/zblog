# zblog

一个自托管的个人博客系统：Rust 单二进制后端（内嵌静态前端），管理后台支持文章编辑与访问统计。

- **后端**：Rust（axum + sqlx + SQLite），位于 `server/`
- **前端**：Astro（静态构建，仅构建期使用）+ Vue 3 组件，位于 `web/`，产物 `web/dist` 嵌入后端二进制
- **架构决策记录**：`docs/adr/`

## 部署拓扑

遵循 ADR-0004：**整个应用部署为单个 Rust 二进制**，Rust 进程是唯一进程，也是公网入口。

- Astro 只是构建期工具（`output: 'static'`）；`rust-embed` 把 `web/dist` 嵌入二进制，axum 以 SPA 回退方式同时伺服静态站点与 API。
- 浏览器所有流量（公开页面、管理页面、API 调用）同源直达 Rust 进程，没有 Node 进程，也没有代理层。
- 管理端认证（密码 + 签名会话 Cookie）在 API 层强制执行；管理与预览页面是公开静态壳，仅由客户端守卫跳转登录，数据边界仍是 API 的鉴权。
- release 构建嵌入静态资源；debug 构建从磁盘上的 `web/dist` 读取，前端重新构建后无需重编译后端。

## 环境变量

| 变量 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `ZBLOG_PASSWORD_HASH` | 是 | 无 | 管理员密码的 Argon2 PHC 哈希。生成方式：`cargo run --example hash_password -- '<密码>'`（在 `server/` 下执行） |
| `ZBLOG_SESSION_SECRET` | 是 | 无 | 会话 Cookie 签名密钥，使用至少 64 个随机字符 |
| `ZBLOG_BIND_ADDR` | 否 | `127.0.0.1:8080` | HTTP 服务监听地址。二进制直接面向公网，生产示例：`0.0.0.0:8080` |
| `ZBLOG_DATABASE_URL` | 否 | `sqlite:zblog.db` | SQLite 连接 URL |

## 构建与运行

### 生产（单二进制）

```sh
cd web && pnpm install && pnpm build && cd ..
cd server && cargo build --release
```

构建顺序不能颠倒：`cargo build` 要求 `web/dist` 已存在（全新克隆时 `server/build.rs` 会生成占位页面，以便单独编译后端）。

运行：

```sh
ZBLOG_PASSWORD_HASH='...' ZBLOG_SESSION_SECRET='...' \
  ZBLOG_BIND_ADDR=0.0.0.0:8080 ./target/release/zblog-server
```

数据库迁移在启动时自动执行（`server/migrations/`）。该二进制自包含静态站点与全部依赖，可单独拷贝到目标主机运行。

### 开发

启动两个进程：后端 API，以及 Astro dev server（vite 会把 `/api` 代理到 `127.0.0.1:8080` 的后端）：

```sh
cd server && ZBLOG_PASSWORD_HASH='...' ZBLOG_SESSION_SECRET='...' cargo run
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
