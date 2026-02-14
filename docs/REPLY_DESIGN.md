# xin reply CLI 设计提案

## 目标

为 `xin reply` 添加 reply 功能，**以 JMAP Email id 为输入**，避免让用户处理 Message-ID：
- Reply to a specific email (via JMAP `emailId`)
- Reply all (auto-populate recipients)
- Preserve threading headers (`In-Reply-To`, `References`)

## CLI 参数设计

参考 gog gmail，但接口更 agent-first：

```
xin reply <emailId> [--reply-all] [--to ...] [--cc ...] [--bcc ...]
          [--subject ...] [--text ... | --body-html ... | --attach ...]
          [--identity <id|email>]
```

### 参数说明

- `<emailId>`：来自 `xin search` / `xin messages search` / `xin inbox next`
- `--reply-all`：在 CC 中补齐原邮件 To + Cc（排除 self）
- `--to`：若提供则不再自动推导 To
- `--subject`：不提供则自动补 `Re: <original subject>`

## 实现原理 (JMAP)

1) **获取原始邮件**: 使用 `Email/get` 读取 `messageId`, `references`, `from`, `to`, `cc`, `subject`
2) **构造 threading headers**:
   - `In-Reply-To: <original-message-id>`
   - `References: <existing-refs> <original-message-id>`
3) **推导收件人**:
   - reply：默认 `From -> To`
   - reply-all：`From -> To` + `To/Cc -> Cc`（排除 self）
4) **创建 draft 并提交**：复用 `Email/set` + `EmailSubmission/set`

## 预期使用方式

```bash
# Basic reply (to sender only)
xin reply <emailId> --text "My reply"

# Reply all
xin reply <emailId> --reply-all --text "My reply"

# Override recipients
xin reply <emailId> --to other@example.com --text "Custom reply"
```

## 输出格式

输出与现有 `xin send` 相同，不破坏现有 JSON schema 约定。

## 测试计划

1. Unit/Mock tests: reply recipient inference + headers
2. Stalwart smoke: reply workflow works end-to-end

## 风险/注意事项

- Threading 依赖 `Message-ID`/`References`，必须从原邮件取值。
- 服务器不应要求用户提供 Message-ID；这是内部实现细节。
