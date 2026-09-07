# Snipe 缺口修复计划

> 依据：`PLAN.md` v0.3、`docs/implementation-status.md`、当前仓库审计（`0.1.1-alpha.4`）。
>
> 性质：这是相对 `PLAN.md` 的  **缺口闭合执行计划**，不是新产品规格。产品目标、里程碑定义、验收指标仍以 `PLAN.md` 为准。
>
> 状态：未开始。完成一项后在对应 checkbox 打勾，并同步更新 `docs/implementation-status.md`。

## 1. 原则

1. **先把主路径做实，再补安全和发布。** 覆盖层看不到截图时，标注、长图、取色都没有可信输入。
2. **库层优先复用。** `domain` / `imaging` / `annotation` / `longshot` / `ai-client` 已有可用实现；本计划以接入、编排和补齐缺口为主，避免重写。
3. **坐标一律走强类型。** 选区、裁剪、覆盖层定位必须经过 `DesktopPx*` ↔ `ImagePx*` ↔ `Logical*`，禁止在 `app` 里继续用裸 `i32` 当桌面坐标。
4. **Slint 回调只提交命令。** 捕获、编码、HTTP、拼接不得在 callback 内同步执行；经应用命令通道 + `spawn_blocking` / Tokio 后台任务完成。
5. **一项一个可验证切片。** 每个任务有文件范围、完成定义和手工验收。未达完成定义不算完成。
6. **不把编译成功当产品完成。** 与 `implementation-status.md` 一致：CI 通过不是 MVP。

## 2. 依赖关系

```text
P0-1 覆盖层显示截图 + 物理坐标
  ├─ P0-2 长截图采集（复用选区）
  ├─ P0-3 标注编辑器（复用裁剪帧）
  ├─ P1-2 放大镜取色（复用帧缓存）
  └─ P1-3 钉图 OCR/翻译（复用选区结果）

P0-4 升级 IPC 握手 ─┐
P1-7 单实例/管道加固 ─┴─ 安装升级路径才可测

P1-1 设置即时生效（热键重注册、开机启动、错误可见）
P1-4 AI 取消/重试/Responses
P1-5 热键冲突提示
P1-6 i18n 接入

P1-8 WGC/DXGI（可与主路径并行，但不能阻塞 P0）
P2-* 配置迁移、资源回收、签名、测试门禁
```

P0 全部完成前，不启动签名、soak、黄金图集等发布门禁工作。

## 3. 阶段总览

| 阶段 | 目标 | 预估 | 完成后产品状态 |
|---|---|---|---|
| Phase 0 | 截图主路径、长图采集、标注编辑器、升级 IPC | 1–2 天切片 × 4 | 能截、能标、能拼、能安全升级退出 |
| Phase 1 | 设置即时生效、取色、钉图菜单、AI、i18n、IPC 加固、WGC | 3–5 天 | 接近 PLAN MVP 功能面 |
| Phase 2 | 配置迁移、资源回收、捕获降级矩阵、发布闭环 | 5–7 天 | 可安装、可诊断、可回滚 |
| Phase 3 | 集成测试、性能、兼容矩阵、soak | 7–10 天 | 达到 `PLAN.md` 发布门禁证据 |

---

## Phase 0 — 紧急主路径

目标：用户按 F1 能看到截图、选出区域、复制/保存/钉图；能标注；能做长截图；安装器升级不会无确认杀进程。

### P0-1 覆盖层显示截图并使用物理坐标

**问题**

- `OverlayWindow` 没有图像属性，背景是半透明黑色，用户看不到捕获帧。
- 选区使用 overlay 逻辑像素，未转换为 `DesktopPxRect` / `ImagePxRect`。
- 覆盖层未按虚拟桌面物理矩形定位、未处理负坐标与混合 DPI。
- `trigger_screenshot` 捕获成功后只 `show()`，没有 `frame_to_slint_image`。

**改动范围**

- `ui/overlay.slint`：增加 `in-out property <image> capture-image`，全屏绘制捕获帧，选区挖空/高亮。
- `crates/ui-slint/src/lib.rs`：保持 `frame_to_slint_image`；必要时提供覆盖层几何 setter。
- `crates/app/src/controller.rs`：捕获后写入图像；记录 `frame_origin: DesktopPxPoint` 与每显示器 `scale_factor`。
- `crates/app/src/controller.rs`：`get_selected_subframe` 改为 `desktop_rect_to_image_rect`。
- `crates/app`：覆盖层窗口位置/尺寸取 `get_virtual_desktop_bounds`；混合 DPI 不稳定时降级为每显示器一个 overlay（`PLAN.md` 5.3）。
- `crates/app`：应用 manifest 在窗口创建前启用 Per-Monitor DPI Awareness V2。

**完成定义**

- [x] Overlay 显示的是本次捕获帧，而不是纯色遮罩。
- [ ] 选区裁剪结果与屏幕像素误差 ≤ 1 物理像素（主屏 100%/150%/200%）；待 tag 打包后的实机验收。
- [ ] 副屏在主屏左侧（负坐标）时裁剪正确；待 tag 打包后的实机验收。
- [ ] 覆盖层、工具栏不进入最终图片；待 tag 打包后的实机验收。
- [x] Esc / 取消隐藏 overlay，不退出进程；`current_frame` 在结束后释放。

**手工验收**

1. F1 → 看到桌面静态截图。
2. 拖出一个矩形 → 尺寸徽章与实际像素一致。
3. 复制到画图 / 保存 PNG，内容与选区一致。
4. 双屏、左屏为副屏时重复 1–3。

### P0-2 长截图采集接到热键和托盘

**问题**

- `HotkeyAction::Longshot` 与 `TRAY_MENU_LONGSHOT` 都调用 `trigger_screenshot`。
- `longshot` crate 只有会话和拼接，没有区域选择、采帧循环、稳定检测、终点、预览恢复。

**改动范围**

- `crates/app/src/main.rs`：Longshot / 托盘菜单分发到 `trigger_longshot`，不再复用截图。
- `crates/app/src/controller.rs`：新增长截图流程：复用 overlay 选区 → `LongshotSession` → 循环 `capture(Region)`。
- `crates/longshot`：补稳定检测、终点（位移≈0 / 无新增 / 用户结束 / 帧数·高度·内存·时长上限）、失败时保留已采集帧。
- `ui/`：最小预览（已拼接高度、帧数、结束/取消）。首版手动滚动必须可用；自动滚轮可放 Phase 1 末或 Phase 2。
- 资源上限先采用 PLAN 建议值并在代码中命名常量：300 帧、30_000 px（后续提到 50_000）、会话超时 5 分钟。

**完成定义**

- [x] Ctrl+Alt+S 与托盘「滚动长截图」进入长图流程，而不是普通截图。
- [x] 用户框选区域后可手动滚动采集，画面稳定后采帧。
- [x] 拼接成功产出一张图，可复制/保存。
- [x] 失败或达上限时不丢已采集帧，给出原因。
- [x] 取消后回到托盘驻留。

**手工验收**

1. 浏览器打开长页面，框选内容区，手动滚动到底，结束并检查接缝。
2. 中途 Esc：已有帧可保存或丢弃，进程仍在。
3. 热键 F1 仍是普通截图，互不抢状态机。

### P0-3 标注编辑器接入

**问题**

- overlay `action-annotate` 未绑定。
- `editor.slint` 只有 `EditorToolbar`，没有画布窗口、没有与 `CommandStack` 的桥。
- 文本栅格是 5x7 ASCII，中文会变成方块；MVP 至少要能输入中文（IME）并在导出时可见。

**改动范围**

- `ui/editor.slint`：完整编辑窗口（原图 + 矢量预览 + 工具栏）。
- `ui/main.slint`：export `EditorWindow`。
- `crates/app`：`on_action_annotate` 打开编辑器，传入裁剪帧。
- `crates/app`：工具回调驱动 `annotation::CommandStack`；完成时 `composite_annotations` 后复制/保存/钉图。
- 首版工具：矩形、椭圆、箭头、画笔、文本、马赛克、序号、撤销/重做。裁剪、对象选中拖拽可在本任务做最小集，精细 hit-test 放到 Phase 1 补丁。
- 文本：编辑态用 Slint `TextInput` + IME；导出可用系统字体栅格或先导出编辑态快照，禁止只输出 5x7 点阵中文。

**完成定义**

- [x] 截图工具栏「标注」打开编辑器，显示选区原图。
- [x] 能添加至少矩形/箭头/画笔/马赛克，撤销/重做有效。
- [x] 完成导出为原图像素分辨率，不按窗口缩放尺寸。
- [x] 取消不修改剪贴板，回到驻留。

### P0-4 升级 IPC 握手与活动任务保护

**问题**

- `--shutdown-for-update` 发送 `SHUTDOWN_FOR_UPDATE`，主实例立刻 `quit_event_loop`。
- 无版本化握手、无 busy/ready、无用户确认、无安装器有界等待协议。
- 管道命令无白名单之外的拒绝语义。

**改动范围**

- `crates/app/src/single_instance.rs`：版本化文本协议（一行 JSON 或 `CMD key=value`），白名单：`OPEN_SETTINGS`、`PING`、`SHUTDOWN_FOR_UPDATE`。
- 状态机：`ready` / `busy` / `cancelled` / `incompatible`。有钉图、标注、长图、AI 任务时回 `busy`。
- `installer/snipe.iss`：升级/卸载先跑 `snipe.exe --shutdown-for-update`，等待有界时间；超时提示手动关闭，不默认强杀。
- 主实例退出前：取消后台任务、关钉图、卸托盘、注销热键（已有 cleanup 需接到 IPC 路径）。

**完成定义**

- [x] 空闲时升级命令使主实例优雅退出并回复 `ready`。
- [x] 长图/标注进行中回复 `busy`，不退出。
- [x] 未知命令被拒绝，不执行。
- [x] 安装器等待超时有明确失败，而不是直接覆盖正在写的文件。

**依赖后续：** P1-7 完成 ACL / 超时 / 身份后，本协议才算生产可用。P0-4 先把语义做对。

---

## Phase 1 — 可用性

### P1-1 设置保存可见失败，热键与开机启动即时生效

**问题**

- `current_cfg.save()` 与 `CredentialStorage::store_secret` 用 `let _ =`，UI 仍写「配置已成功保存」。
- 保存热键后不 `UnregisterHotKey` / `RegisterHotKey`。
- `auto_start` 不写 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`。
- 保存目录、格式、浏览按钮未接线。
- F2 钉图热键未注册。

**改动范围**

- `crates/app/src/controller.rs`、`crates/app/src/hotkey.rs`、`crates/app/src/main.rs`。
- 新增开机启动服务（HKCU Run），安装器只传递首次选择，之后以应用设置为准（`PLAN.md` 3.6）。
- `ui/settings.slint`：保存目录/格式接线；失败状态红色可见。

**完成定义**

- [ ] 保存失败时 UI 显示错误，不显示成功。
- [ ] 改 F1 后无需重启即可用新热键；旧热键失效。
- [ ] 冲突时提示，并保留旧热键。
- [ ] 开机启动开关与注册表一致；卸载可删除该值。
- [ ] F2 按配置注册，不再落到截图（若产品确认 F2 为钉图；否则从设置中移除该误导项）。

### P1-2 完整取色器

**问题**

- `trigger_color_picker` 只是再开一次 overlay。
- `imaging::generate_magnifier` 未接到 UI。
- 无 HEX/RGB/HSL/HSV 切换与点击复制。

**改动范围**

- overlay 或独立取色层：像素网格放大镜、中心十字、格式切换。
- 取色使用已捕获帧，不在鼠标移动时反复整屏捕获。
- 快捷键 F3 进入取色模式，与截图模式状态机分离（`AppMode::ColorPicking`）。

**完成定义**

- [ ] 放大镜显示真实邻域像素。
- [ ] 可复制 `#RRGGBB`、`#RRGGBBAA`、`rgb()`、`hsl()`、`hsv()`。
- [ ] 混合 DPI 下取到的是该屏物理像素色。

### P1-3 钉图交互补齐

**问题**

- 无拖动、缩放、透明度、置顶切换、鼠标穿透与恢复。
- `ocr-clicked` / `translate-clicked` / `adjust-opacity` 未绑定。
- 无真正右键菜单。

**改动范围**

- `ui/pin.slint`、`crates/app/src/pin_manager.rs`。
- HWND 补置顶、透明度、WS_EX_TRANSPARENT；穿透后必须能用全局热键或托盘恢复。
- OCR/翻译复用 `perform_ai_task`，使用原图像素而非预览缩放图。

**完成定义**

- [ ] 同时 ≥5 个钉图，互不阻塞。
- [ ] 拖动、滚轮缩放、透明度可调。
- [ ] 穿透可恢复。
- [ ] 右键或工具条可复制、保存、OCR、翻译、关闭。
- [ ] 关闭后释放该钉图图像引用。

### P1-4 AI：取消、重试、Responses、凭据绑定

**问题**

- 仅 Chat Completions；`max_retries` 未使用。
- UI 无取消；`CancellationToken` 未暴露。
- Key 固定 `Snipe/AI/ApiKey`，换 Base URL 可能误用旧凭据。
- 无请求大小限制、压缩、长图分块、隐私来源展示、日志脱敏测试。

**改动范围**

- `crates/ai-client`：Responses 适配器 + Chat Completions 保留；错误分类保持。
- 重试仅对超时/429/可恢复网络错误；401 不重试。
- 结果窗增加取消；取消后不更新已销毁窗口。
- Credential target = 规范化 `(provider, origin)`。
- 上传前 UI 显示目标 origin；日志禁止完整 Key、Base64 图。

**完成定义**

- [ ] 官方 Responses 与至少一个已记录的 compatible Chat Completions 端点有契约测试。
- [ ] 取消、超时、429、错误 Key 均有用户可读错误且不泄露 Key。
- [ ] 换 Base URL 后不读取上一来源的 Key。

### P1-5 热键冲突与暂停

**问题**

- 注册失败只 `warn!`。
- 托盘「暂停快捷键」在 PLAN 中存在，代码中没有。

**完成定义**

- [ ] 冲突在设置页和托盘可感知。
- [ ] 可暂停/恢复全部全局热键，不退出进程。

### P1-6 i18n 真正接入

**问题**

- `assets/i18n/zh-CN.json`、`en-US.json` 未加载。
- 托盘菜单、overlay、settings、AI 文案硬编码。

**完成定义**

- [ ] `config.ui.language` 切换中/英后，托盘、设置、overlay、结果窗使用对应文案。
- [ ] 不再新增硬编码用户可见字符串（平台错误码可例外）。

### P1-7 单实例对象作用域与管道加固

**问题**

- 互斥量 `Local\Snipe_SingleInstance_Mutex`、管道 `\\.\pipe\Snipe_IPC_Pipe` 未按用户 SID/会话加盐。
- 无 current-user ACL、无读超时、输入无界。

**完成定义**

- [ ] 对象名包含用户/会话作用域。
- [ ] 管道 DACL 仅当前用户；他用户连接失败。
- [ ] 单帧读取有超时和长度上限；非法 UTF-8 / 未知命令拒绝。
- [ ] 二次启动只唤醒已有实例，不出现第二个托盘图标。

### P1-8 捕获后端：能力探测 + WGC/DXGI 路径

**说明：** 不阻塞 Phase 0。GDI 作为兼容回退保留（ADR 0002）。

**完成定义**

- [ ] 运行时探测 WGC / DXGI / GDI，按优先级选择。
- [ ] 受保护内容、UAC 安全桌面、权限不足返回明确错误，不产出空白图死循环。
- [ ] 远程桌面至少有一条已测试路径或明确不支持说明。

---

## Phase 2 — 工程与发布基础

### P2-1 配置 schema 与校验

- [ ] `schema_version` 向前迁移。
- [ ] 热键字符串、质量范围、路径、URL 校验；坏文件回退默认并备份。

### P2-2 钉图与截图资源回收

- [ ] 关闭 overlay / 钉图 / 编辑器后释放 `Frame` 与 Slint `Image`。
- [ ] 空闲不长期持有全桌面帧。
- [ ] 记录 10 钉图内存基线（PLAN 11.5）。

### P2-3 错误可诊断

- [ ] 捕获/保存/AI 失败有用户可操作文案 + 日志关联 id。
- [ ] 本地诊断包（版本、OS、显示器拓扑、后端、不含截图内容与 Key）。

### P2-4 安装生命周期

- [ ] 覆盖升级：P0-4 + P1-7 协议接入 `snipe.iss`。
- [ ] 卸载可选删除配置与 Credential Manager 条目。
- [ ] `scripts/smoke-test.ps1` 覆盖安装/升级/卸载，而不是只跑 `--shutdown-for-update`。

### P2-5 签名与许可证

- [ ] 主程序与安装包签名 + RFC 3161 时间戳。
- [ ] 第三方许可证清单随包。
- [ ] SHA-256 校验文件由发布流程生成（已有 checksum step，需与签名顺序对齐）。

### P2-6 DPI manifest 与窗口消息

- [ ] Per-Monitor V2 manifest。
- [ ] `WM_DPICHANGED`、显示器插拔、`TaskbarCreated`、睡眠唤醒后刷新坐标与热键（托盘重建已有雏形）。

---

## Phase 3 — 测试与门禁证据

未完成不得将预发布标为稳定版。

### P3-1 自动化

- [ ] 坐标往返、裁剪边界、命令栈、文件名模板、AI 错误映射、拼接位移：单元测试补齐。
- [ ] AI mock：成功、超时、429、500、非法 JSON、取消、重定向。
- [ ] IPC：合法命令、非法命令、busy、超时。

### P3-2 黄金图与长图样本

- [ ] 箭头/矩形/马赛克/模糊导出黄金图（允许抗锯齿阈值）。
- [ ] 至少 30 个长图样本，成功率 ≥ 90%（PLAN 1.3 / M5）。

### P3-3 性能与 soak

- [ ] 快捷键到 overlay 可交互 P95 < 150 ms（驻留态，≥500 次）。
- [ ] 10_000 次保存成功率 ≥ 99.9%，失败可诊断。
- [ ] 72 h 后台：热键与托盘仍可用，无持续句柄/GDI/内存增长。
- [ ] 空闲内存基线写入 `docs/ui-poc-report.md` 并冻结门槛。

### P3-4 兼容矩阵

- [ ] Win10 22H2、Win11 当前支持版；本地与远程。
- [ ] 单屏 100/125/150/200%；双屏混合 DPI；副屏在左/上。
- [ ] HDR 限制写入文档，不假装支持。

---

## 4. 任务与代码锚点

| ID | 主要文件 |
|---|---|
| P0-1 | `ui/overlay.slint`, `crates/app/src/controller.rs`, `crates/ui-slint/src/lib.rs`, `crates/domain/src/coordinates.rs`, `crates/capture-windows/src/display.rs`, app manifest |
| P0-2 | `crates/app/src/main.rs`, `crates/app/src/controller.rs`, `crates/longshot/src/*.rs` |
| P0-3 | `ui/editor.slint`, `ui/main.slint`, `crates/app/src/controller.rs`, `crates/annotation/src/*.rs` |
| P0-4 | `crates/app/src/single_instance.rs`, `crates/app/src/main.rs`, `installer/snipe.iss` |
| P1-1 | `controller.rs`, `hotkey.rs`, `ui/settings.slint`, 新增 startup/registry 模块 |
| P1-2 | `overlay.slint` 或新 `picker.slint`, `imaging/src/magnifier.rs`, `controller.rs` |
| P1-3 | `ui/pin.slint`, `crates/app/src/pin_manager.rs` |
| P1-4 | `crates/ai-client/src/*`, `controller.rs`, `ui/ai-result.slint`, `secure-storage-windows` |
| P1-6 | `assets/i18n/*`, tray/settings/overlay 字符串入口 |
| P1-7 | `single_instance.rs` |
| P1-8 | `capture-core`, `capture-windows` |
| P2-4 | `installer/snipe.iss`, `scripts/smoke-test.ps1`, `scripts/package.ps1` |
| P3-* | `tests/`, `docs/ui-poc-report.md`, CI |

## 5. 明确不在本计划首轮做的事

与 `PLAN.md` 2.2 / 2.3 一致，避免范围膨胀：

- 截图历史、钉图分组/圆角/旋转、标注图层面板
- OCR 位置框、双语对照、长图接缝手修
- 二维码、文件拖入、账号云同步、插件
- 本地 OCR 模型、录屏/GIF
- macOS/Linux、便携版、MSIX、Store
- 自动更新（首版最多手动检查）

## 6. 执行约定

1. 按 Phase 0 → 1 → 2 → 3 顺序；Phase 内按 ID 顺序，除非依赖图允许并行（P1-6 可与 P1-2/P1-3 并行；P1-8 可与 P1 其它项并行）。
2. 每完成一个 ID：更新本文件 checkbox，并改 `docs/implementation-status.md` 对应里程碑证据。
3. 行为变更若与 `PLAN.md` 冲突，先改 PLAN 再改代码。
4. 公开 Tag 失败后不得复用同一 Tag；升 prerelease/patch。
5. 稳定版 Tag 前必须满足：Phase 0–1 完成定义全部勾选，Phase 3 门禁有附件证据，且 `implementation-status.md` 不再是 NO-GO。

## 7. 文档关系

| 文档 | 角色 |
|---|---|
| `PLAN.md` | 产品规格与里程碑 |
| `docs/architecture.md` | 分层与 crate 职责 |
| `docs/implementation-status.md` | 当前审计结论（NO-GO） |
| `docs/repair-plan.md` | 本文件：缺口 → 任务 → 验收 |
| `docs/adr/*` | 已冻结决策（Slint、GDI 基线） |
