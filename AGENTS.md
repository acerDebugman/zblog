# zblog - AI Agent Guide

## Project Overview

zblog 是一个博客项目。

- **Backend**: Rust
- **Frontend**: Astro + Vue

> 项目尚在初始阶段，目录结构、构建命令等随开发进展补充。

## Directory Layout

- `server/` — Rust 后端（axum + sqlx + SQLite），`src/` 为源码，`tests/` 为集成测试，`migrations/` 为数据库迁移，`examples/hash_password.rs` 用于生成管理员密码哈希；`build.rs` 在 `web/dist` 缺失时生成占位页面，以便全新克隆能单独编译后端；release 构建通过 `rust-embed` 把 `web/dist` 嵌入二进制（debug 构建从磁盘读取），单二进制部署见 ADR-0004
- `web/` — Astro 静态构建（`output: 'static'`，仅构建期使用，无 Node 运行时）+ Vue 3 前端，`src/pages/` 为路由（静态壳，客户端守卫），`src/components/` 为 Vue 组件，`src/lib/` 为 API 客户端与 Markdown 渲染管线
- `docs/adr/` — 架构决策记录

## Build & Test Commands

统一入口为仓库根目录的 `Makefile.toml`（cargo-make），在根目录执行：

- 后端开发: `cargo make dev`（需 `ZBLOG_PASSWORD_HASH`、`ZBLOG_SESSION_SECRET` 环境变量）
- 前端开发: `cargo make dev-web`（vite 将 `/api` 代理到 `127.0.0.1:8080`）
- 全部测试: `cargo make test`（后端 `cargo test` + 前端 `pnpm vitest run`）
- 全部检查: `cargo make check`（`cargo clippy --all-targets` + `pnpm astro check`）
- 本地生产构建: `cargo make build`（必须先构建前端，`web/dist` 会被嵌入二进制）
- 发布打包: `cargo make package`（在 `zblog:dev` manylinux 镜像内编译，产物为 `dist/zblog-server` + `.sha256`；容器内以 root 编译后 chown 回宿主机用户）
- 生成密码哈希: `cargo make hash-password`
- 列出全部任务: `cargo make --list-all-steps`

## Technology Stack

### Backend

- **Language**: Rust 1.90+
- **Async Runtime**: Tokio

### Frontend

- **Framework**: Astro + Vue 3 (TypeScript)
- **Language**: TypeScript（严格模式）

## Code Style Guidelines

### Rust Code Style

1. **严格 Lint 规则**（建议在 workspace `Cargo.toml` 中统一定义）:

   ```toml
   [lints.rust]
   unsafe_code = "forbid"
   warnings = "deny"

   [lints.clippy]
   all = { level = "deny", priority = -1 }
   pedantic = { level = "warn", priority = -1 }
   nursery = { level = "warn", priority = -1 }
   unwrap_used = "deny"
   expect_used = "deny"
   panic = "deny"
   ```
2. **错误处理**:

   - 禁止使用裸 `unwrap()`, `expect()`, `panic!()`
   - 使用统一的错误枚举封装错误（如 `AppError`）
   - 使用 `crate::Result<T>` 作为统一返回类型
   - 使用 `#[must_use]` 标记重要返回值
3. **文档注释**:

   - 所有 public API 必须有 `///` 文档注释
   - 复杂函数需要 `# Errors` 和 `# Panics` 说明
   - 使用 `//!` 为模块添加文档
4. **模块组织**:

   - 遵循清晰分层（如 domain -> application -> infrastructure -> interfaces）
   - 每层通过 `mod.rs` 暴露公共接口
   - 依赖方向: 上层 -> 下层 (domain 是最底层)
5. **命名规范**:

   - 结构体/枚举: PascalCase
   - 函数/变量: snake_case
   - 常量: SCREAMING_SNAKE_CASE
   - 类型别名: PascalCase
6. **语言**:

   - 所有的文字描述或者返回值，错误信息打印等都使用英文

### TypeScript/Vue Code Style

1. **严格 TypeScript 模式**
2. **Composition API**: 使用 `<script setup>` 语法
3. **类型定义**: 优先使用 `type` 而非 `interface`
4. **Props/Emit**: 必须显式定义类型
5. **运行时校验**: 使用 zod 进行数据校验

## Testing Strategy

### Rust 测试

```rust
// 单元测试示例 (在源码文件中)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = MyEntity::new("Test").unwrap();
        assert_eq!(entity.name, "Test");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_operation().await;
        assert!(result.is_ok());
    }
}
```

- 单元测试与源码放在同一文件，使用 `#[cfg(test)]`
- 集成测试放在 `tests/` 目录
- 不要只用数量验证：要用返回数据的内容验证
- 使用互斥的测试数据：确保搜索结果可以明确区分是否过滤正确
- 使用严格的等于断言：不要用 >= 或 <=
- 验证反向条件：验证不匹配的确实没有返回

### Frontend 测试

```typescript
// 组件测试
import { mount } from '@vue/test-utils'
import MyComponent from './MyComponent.vue'

describe('MyComponent', () => {
  it('renders message', () => {
    const wrapper = mount(MyComponent, {
      props: { message: 'Hello' }
    })
    expect(wrapper.text()).toContain('Hello')
  })
})
```

## Git Commit Convention

使用 Conventional Commits 格式：

```
类型(范围): 描述

允许类型: feat, fix, docs, style, refactor, test, chore

示例:
- feat(blog): 添加文章评论功能
- fix(api): 修复文章查询返回空值的问题
- docs(readme): 更新安装说明
```

## 约束

1. UI 页面显示上中文和英文要有空格
