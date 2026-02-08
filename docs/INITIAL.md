# xin（信）— JMAP CLI 对标 `gog gmail` 的初始规格

目标很明确：

- **`gog gmail` 负责 Gmail**（保持不动）
- **`xin` 负责 JMAP**（Fastmail 起步，但做成通用 JMAP CLI）
- `xin` 的命令接口尽量 **对标 / 类似 `gog gmail`**，让人和 agent 都能无脑迁移使用习惯

> xin 不是 gog 的替代，也不是统一多邮箱的 wrapper。
> “汇总多个邮箱”可以在未来做一个更上层的 orchestrator（例如另一个命令/脚本），但不放在 xin v1 的 scope。

---

## 1. 我们对标的 `gog gmail` 命令面（基线）

（以下基于本机 `gog` v0.9.0 帮助输出整理）

### 1.1 Read
- `gog gmail search <query> --max N [--page TOKEN] [--oldest]`  （threads）
- `gog gmail messages search <query> --max N`  （messages）
- `gog gmail get <messageId> [--format full|metadata|raw] [--headers ...]`
- `gog gmail thread get <threadId> [--download] [--full] [--out-dir ...]`
- `gog gmail thread attachments <threadId>`
- `gog gmail attachment <messageId> <attachmentId> [--out ...] [--name ...]`
- `gog gmail url <threadId>...`
- `gog gmail history [--since HISTORY_ID] [--max N] [--page TOKEN]`

### 1.2 Organize
- `gog gmail thread modify <threadId> --add LABELS --remove LABELS`
- `gog gmail batch modify <messageId>... --add LABELS --remove LABELS`
- `gog gmail batch delete <messageId>...`（永久删除）

### 1.3 Labels
- `gog gmail labels list|get|create`
- `gog gmail labels modify <threadId>... --add ... --remove ...`

### 1.4 Write
- `gog gmail send ...`（reply/thread/reply-all/attach 等）
- `gog gmail drafts list|get|create|update|delete|send`

### 1.5 额外能力（Gmail-specific）
- `gog gmail track setup|opens|status`（gog 自己做的 tracking feature）
- `gog gmail settings ...`（filters/watch/delegates/sendas/vacation/forwarding…）

这些 Gmail 专属的 admin 面，xin **不对标**（因为 JMAP 没有跨服务商的等价物）。

---

## 2. `xin` 的命令面：对标 `gog gmail`，但面向 JMAP

### 2.1 全局设计原则（agent-first）

- 默认输出：**稳定 JSON**（便于 agent 解析）
- 安全：默认只读；会改动状态的命令需要显式执行（并提供 `--dry-run` 和/或 `--yes`）
- 认证：
  - MVP：支持 Fastmail API token / app password（Basic/OAuth token 视服务端）
  - 通用：后续加 OAuth（但不阻塞 v0）
- 配置：支持多 account（多个 JMAP server/多个邮箱），但 **xin 本身只负责 JMAP**

### 2.2 具体命令（建议尽量贴近 gog gmail 的语义）

> 下面以“对标”角度列出：gog 的命令 → xin 里的对应命令。

#### Read

1) `gog gmail search`（threads 搜索）
- **xin 对标**：`xin search <query> [--max N] [--page TOKEN] [--oldest]`
- 说明：
  - Gmail 的 `<query>` 是 Gmail query language；
  - xin 的 `<query>` 应该是 **portable filter DSL**（或 `--filter-json` 直接传 JMAP filter）。

2) `gog gmail messages search`（messages 搜索）
- **xin 对标**：`xin messages search <query> ...`

3) `gog gmail get <messageId>`
- **xin 对标**：`xin get <emailId> [--format full|metadata|raw]`
- JMAP 中实体通常是 `Email`；必要时 `--raw` 输出 JMAP 原始对象。

4) `gog gmail thread get <threadId>`
- **xin 对标**：`xin thread get <threadId> [--full]`
- 风险：并非所有 JMAP server 都把 thread 做得和 Gmail 一样强；需要 capability 检测与 fallback。

5) `gog gmail attachment / thread attachments`
- **xin 对标**：
  - `xin thread attachments <threadId>`
  - `xin attachment <emailId> <blobId> [--out ...]`
- JMAP 通常是 blob/download URL 模型。

6) `gog gmail url <threadId>...`
- **xin 对标**：`xin url <id>...`（可选）
- 说明：JMAP 标准不保证 webmail URL，Fastmail 可能可以拼，但其他 provider 未必。

7) `gog gmail history`
- **xin 对标**：`xin history [--since <state>] [--max N] [--page TOKEN]`
- 对应：JMAP 的 `Email/changes`（或相关 changes）提供增量变更；用 state token。

#### Organize

8) `gog gmail thread modify --add/--remove LABELS`
- **xin 对标**：`xin thread modify <threadId> --add <tags> --remove <tags>`
- 映射：
  - Gmail label ≈ JMAP keyword（例如 `$seen/$flagged`）+ mailbox membership
  - xin 需要提供统一抽象：`INBOX/ARCHIVE/TRASH/UNREAD/STARRED` 等。

9) `gog gmail batch modify <messageId>...`
- **xin 对标**：`xin batch modify <emailId>... --add ... --remove ...`

10) `gog gmail batch delete`（永久删除）
- **xin 对标**：`xin batch delete <emailId>...`（默认不鼓励；更推荐 `trash`）
- v0 可先只实现 `trash`，把 delete 放到后续。

#### Labels

11) `gog gmail labels list|get|create`
- **xin 对标**：`xin labels list|get|create`
- 映射：
  - Gmail label
  - JMAP: `Mailbox`（文件夹）+ `keyword`（标签）
- 建议：xin 的 `labels` 主要对标“mailboxes”，keywords 作为 `tags/keywords` 一套。

12) `gog gmail labels modify ...`
- **xin 对标**：`xin labels modify <id>... --add ... --remove ...`

#### Write

13) `gog gmail send ...`
- **xin 对标**：`xin send ...`（支持 `--reply-to-email-id` / `--thread-id` / `--reply-all`）
- JMAP 通常为：`Email/set` 生成 draft + `EmailSubmission/set` 提交发送。

14) `gog gmail drafts ...`
- **xin 对标**：`xin drafts list|get|create|update|delete|send`
- drafts 在 JMAP 里一般是一个 mailbox（Drafts）。

---

## 3. Gmail vs JMAP：哪些我们做不到“等价”，哪些可能更强

### 3.1 难以等价对标的点（限制）

- **Gmail query language**：JMAP 的 filter 是结构化 JSON；不同 provider 的全文检索能力差异大。
- **Gmail Categories/Importance**：Promotions/Updates/Primary、Important 等信号不可移植。
- **Gmail settings/admin**：filters/watch(PubSub)/delegates/sendas/vacation/forwarding 等没有通用 JMAP 等价。
- **Gmail web URL**：JMAP 不保证提供 webmail URL。

### 3.2 我们可能超越的点（机会）

- **批处理能力更一致**：JMAP 设计本来就鼓励一次 request 做多个变更（methodCalls/backreference）。
- **增量同步更“标准化”**：`*/changes` +（部分支持）WebSocket push（RFC 8887）比 Gmail watch 更容易做成通用接口。
- **输出更 agent-friendly**：从第一天就定义稳定 JSON schema（而不是“人类 CLI 输出”再反解析）。
- **跨 provider 的一致语义**：虽然 xin 只做 JMAP，但在 JMAP 世界内部（Fastmail/Stalwart/Cyrus/James…）我们可以做到一致。

---

## 4. 开发计划（以“gog gmail parity”为导向）

### Phase 0 — 选型与对齐
- 选一个成熟 JMAP client library（Go 或 Rust）
- 验证覆盖：`Email/query`, `Email/get`, `Email/set`, `Mailbox/get`, `EmailSubmission/set`, `*/changes`
- 设计 xin 的稳定 JSON 输出 schema（threads/emails/mailboxes/attachments/errors）

### Phase 1 — Read parity（最先让 agent 能“看见”邮件）
- `xin search`（支持简易 DSL + `--filter-json`）
- `xin messages search`
- `xin get` / `xin thread get`
- `xin attachment` / `xin thread attachments`

### Phase 2 — Organize parity（安全地改状态）
- `xin thread modify` / `xin batch modify`
- `xin archive`（作为 `modify --remove INBOX` 的 sugar）
- `xin read`（作为 `modify --add $seen` 的 sugar）
- `xin trash`（移动到 Trash mailbox）

### Phase 3 — Write parity（发信闭环）
- `xin drafts ...`
- `xin send`（含 reply / reply-all / thread）

### Phase 4 — History / watch（可选增强）
- `xin history`（`*/changes`）
- `xin watch`（如果 provider 支持 RFC 8887 或其它 push，做成可选模块；不强求）

---

## 5. 明确不做（v1）

- Gmail-only settings/admin 的对标
- “统一检查 Gmail + JMAP 并汇总”的上层 orchestrator（可以另开一个项目/命令做）
- 试图复刻 Gmail 的类别/重要性模型
