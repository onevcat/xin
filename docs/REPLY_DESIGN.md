# xin reply CLI 设计提案

## 目标

为 `xin send` 添加 reply 功能，支持：
- Reply to a specific email (via Message-ID)
- Reply all (auto-populate recipients)
- Reply within a thread

## CLI 参数设计

参考 gog gmail 和 INITIAL.md，设计如下：

### 新增参数

```rust
/// Reply to a specific email. Sets In-Reply-To and References headers
/// based on the original email's Message-ID.
#[arg(long = "reply-to-message-id")]
pub reply_to_message_id: Option<String>,

/// Reply within a thread. The thread is determined by the original email's
/// thread or explicitly via thread ID. Sets References header.
#[arg(long)]
pub thread_id: Option<String>,

/// Auto-populate recipients from original message (To -> To, CC -> CC).
/// Requires --reply-to-message-id or fetching from a thread.
#[arg(long)]
pub reply_all: bool,

/// Custom Reply-To header address.
#[arg(long = "reply-to")]
pub reply_to: Option<String>,
```

### 参数互斥/依赖

- `--reply-all` 必须配合 `--reply-to-message-id` 或 `--thread-id`（或从 thread 解析）
- `--thread-id` 在 JMAP 中是服务器管理的，客户端不能直接设置。但可以通过 `References` header 来关联线程。
- `--reply-to-message-id` 是最核心的参数，用于获取原始邮件的 Message-ID

### 实现原理 (JMAP)

1. **获取原始邮件**: 使用 `Email/get` 获取原始邮件的 metadata
2. **提取 headers**:
   - `messageId` -> 设置 `In-Reply-To` header
   - `references` (如果有) + `messageId` -> 设置 `References` header
3. **自动填充 recipients** (当使用 `--reply-all`):
   - Original `to` -> New `cc`
   - Original `cc` -> New `cc`
   - Original `from` -> New `to`

## 预期使用方式

```bash
# Basic reply (to sender only)
xin send --reply-to-message-id "<original-message-id@example.com>" \
  --subject "Re: Original Subject" --text "My reply"

# Reply all
xin send --reply-to-message-id "<original-message-id@example.com>" \
  --reply-all --subject "Re: Original Subject" --text "My reply"

# Reply with custom Reply-To
xin send --reply-to-message-id "<original-message-id@example.com>" \
  --reply-to "another@example.com" --subject "Re: Original Subject" --text "My reply"
```

## 输出格式

输出与现有 `xin send` 相同，不破坏现有 JSON schema 约定。

## 测试计划

1. Unit tests: 参数解析、header 构建逻辑
2. Smoke tests: 模拟回复流程（需要 mock JMAP server）

## 风险/注意事项

- `--thread-id` 在 JMAP 中是隐式的，服务器自动管理。我们只能通过 `References` header 来"建议"线程关联。
- 如果原始邮件没有 `Message-ID`，需要报错或警告。
- 需要考虑安全问题：防止循环引用等。
