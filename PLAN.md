# Rust 截图工具项目实施计划

> 定位：使用 Rust 开发一款面向 Windows 10/11 的高效率截图工具，核心体验参考 PixPin，但不复制其产品、代码或视觉资产。
>
> 核心能力：常规截图、长截图、钉图、截图标注、OCR 识图、截图翻译、颜色拾取。
>
> 文档状态：v0.3（最终审查版；允许进入 M0）  
> 首发平台：Windows 10/11 x64  
> UI 技术：Slint + Rust，特殊窗口与系统能力通过 `windows` crate 接入 Win32  
> 运行方式：单实例、系统托盘后台常驻，全局快捷键唤起功能  
> 安装方式：Inno Setup 生成单文件 `.exe` 安装包  
> AI 能力：支持 OpenAI 及 OpenAI-compatible API，自定义 Base URL、模型与 API Key

---

## 1. 项目目标

### 1.1 产品目标

构建一个启动快、占用低、操作连贯、隐私边界清晰的桌面截图工具。软件安装并启动后以单实例方式在系统托盘后台常驻；关闭截图、设置或结果窗口不退出主进程，用户可通过全局快捷键随时唤起功能。用户应能够完成以下完整链路：

1. 唤起截图覆盖层；
2. 选择屏幕区域、窗口或全屏；
3. 立即复制、保存、标注、钉图；
4. 对截图执行 OCR 或翻译；
5. 在取色模式中获取 HEX、RGB、HSL 等颜色值；
6. 对可滚动区域生成长截图。

### 1.2 工程目标

- 核心业务逻辑尽量使用 Rust 实现。
- Windows 平台能力与业务逻辑分层，便于后续扩展 macOS/Linux。
- 截图、标注、长截图、AI 服务之间采用清晰接口，避免相互耦合。
- API Key 不以明文写入普通配置文件或日志。
- 在多显示器、不同 DPI、缩放比例和 HDR/SDR 混合场景下保持坐标准确。
- 核心截图功能不得依赖网络；OCR 和翻译失败不能阻塞截图主流程。
- 后台常驻时不创建不可见的大型主窗口，不长期持有截图和 GPU 纹理。
- 可见窗口全部关闭后，全局快捷键、托盘菜单和任务调度仍然可用。
- 通过单个 Inno Setup `setup.exe` 完成用户级安装、升级和卸载。

### 1.3 首版成功指标

- 冷启动后首次截图可在 500 ms 内进入可交互状态（目标值；测试硬件、系统、显示配置和测量起止点在 M0 固化）。
- 截图覆盖层出现耗时 P95 小于 150 ms（后台驻留状态；至少 500 次样本）。
- 选区坐标在 100%、125%、150%、175%、200% DPI 下误差不超过 1 个物理像素。
- 普通截图在 10,000 次自动保存压力测试中的成功率不低于 99.9%，失败必须产生可诊断错误。
- 长截图测试集首版至少包含 30 个静态/固定头部/长列表样本：无明显重复、缺行或错位的结果占比不低于 90%；其余场景可安全降级或提示。
- OCR/翻译服务配置错误时提供明确提示，且不泄露 API Key。
- 后台空闲内存目标小于 80 MB；Slint 验证性 PoC 完成后记录基线并冻结正式门槛。
- 后台连续运行 72 小时后，全局快捷键、托盘菜单仍可用，且无持续性句柄/GDI/GPU/内存增长。
- 重复启动程序不会出现第二个托盘图标或重复注册快捷键。
- 正式版本交付一个签名的 `.exe` 安装程序，可完成安装、覆盖升级、卸载和可选开机启动。

---

## 2. 范围定义

## 2.1 MVP（必须完成）

### A. 常规截图

- 全局快捷键唤起。
- 区域截图。
- 当前屏幕全屏截图。
- 鼠标所在窗口自动识别与选择。
- 选区移动、缩放、取消、确认。
- 显示选区尺寸与鼠标位置颜色。
- 截图后操作：
  - 复制到剪贴板；
  - 保存为 PNG/JPEG/WebP；
  - 打开标注编辑器；
  - 钉到桌面；
  - 发起 OCR；
  - 发起翻译。
- 单实例后台常驻；关闭可见窗口后继续运行。
- 托盘菜单：截图、长截图、取色、打开设置、暂停快捷键、退出。
- 可配置全局快捷键并提供冲突提示。
- 可选开机启动，支持在设置页启用或关闭。
- 仅通过托盘“退出”、经用户确认的升级 IPC，或系统会话结束正常终止程序；关闭普通窗口不退出。

### B. 截图标注

- 矩形、椭圆、直线、箭头。
- 自由画笔、马克笔。
- 文本。
- 马赛克与模糊。
- 序号标记。
- 撤销、重做、删除、复制。
- 颜色、线宽、透明度、字体大小设置。
- 裁剪与画布调整。
- 标注结果导出。

### C. 钉图

- 将截图显示为无边框置顶窗口。
- 拖动、缩放、透明度调整。
- 置顶开关、穿透鼠标开关。
- 右键菜单：复制、保存、OCR、翻译、关闭。
- 支持同时存在多个钉图窗口。
- 重启后恢复钉图属于增强项，MVP 可不恢复。

### D. 颜色拾取

- 屏幕像素级取色。
- 放大镜预览。
- 输出 HEX、RGB、HSL、HSV。
- 点击复制当前格式。
- 快捷键切换格式。
- 正确处理多屏和 DPI。

### E. OCR 与截图翻译

- 支持 OpenAI 官方 API。
- 支持 OpenAI-compatible 服务：自定义 Base URL、API Key、模型名。
- 使用具备视觉能力的多模态模型直接识别图片文字。
- OCR 输出纯文本，并支持复制。
- 翻译支持目标语言、保留段落、尽量保留排版语义。
- 请求超时、取消、重试、限流错误处理。
- 上传前明确显示目标服务商；设置中提供隐私提示。
- API Key 存入 Windows Credential Manager，不写入普通配置或日志。

### F. 长截图基础版

- 用户框选可滚动区域。
- 手动滚动捕获模式必须可用。
- 自动滚动优先支持 Chromium/Electron 类窗口及常见 Win32 滚动容器。
- 根据重叠区域进行图像配准和拼接。
- 检测固定头部、重复区域、滚动终点和拼接失败。
- 拼接前后提供预览、裁剪与保存。

## 2.2 v1.x 后续增强项（不阻塞首个正式版）

- 截图历史（默认关闭或明确配置保存策略）。
- 钉图分组、阴影、圆角、旋转。
- 标注对象图层面板。
- OCR 结果与图片位置对应的文本框。
- 翻译后的双语对照视图。
- 长截图局部修正与接缝手动调整。
- 二维码识别。
- 文件拖入后直接标注/OCR/钉图。
- 自动命名模板。
- 静默安装参数与企业部署支持。

## 2.3 暂不纳入首版

- 视频录屏、GIF 录制。
- 云同步与账号体系。
- 团队协作、在线分享链接。
- macOS/Linux 正式支持。
- 浏览器插件。
- 依赖注入式第三方插件系统。
- 本地 OCR 模型打包。
- 免安装便携版、MSIX 和 Microsoft Store 发布。

---

## 3. 关键用户流程

## 3.1 快速截图

1. 用户按下全局快捷键。
2. 程序捕获虚拟桌面或当前显示器画面。
3. 显示半透明覆盖层，识别鼠标下窗口。
4. 用户单击选择窗口，或拖动创建区域。
5. 用户可直接按 Enter/双击确认，按 Esc 取消。
6. 根据默认动作复制到剪贴板，也可选择保存、标注或钉图。

验收重点：覆盖层不得被截入最终图片；光标、选区和实际像素坐标必须一致。

## 3.2 标注并复制

1. 截图后进入轻量标注模式。
2. 用户绘制箭头、矩形、文字、马赛克。
3. 所有操作进入命令栈，可撤销/重做。
4. 确认后离屏渲染合成结果并复制。

验收重点：编辑期间保留原图；重复编辑不得累积压缩损失。

## 3.3 钉图

1. 截图完成后点击“钉图”。
2. 创建独立无边框窗口并显示原图。
3. 用户拖动窗口、滚轮缩放、快捷键调透明度。
4. 右键执行 OCR/翻译或关闭。

验收重点：多个钉图窗口互不阻塞；透明度与鼠标穿透可恢复。

## 3.4 OCR/翻译

1. 用户在截图结果或钉图菜单中选择 OCR/翻译。
2. 后台任务压缩图片并按配置调用视觉模型。
3. UI 显示可取消的加载状态。
4. 成功后展示文本或翻译结果；失败时显示可操作的错误信息。
5. 用户复制结果或保存为文本。

验收重点：网络请求不阻塞 UI；取消后不再更新已销毁窗口。

## 3.5 长截图

1. 用户框选滚动容器区域。
2. 程序固定捕获区域，提示用户手动滚动或授权自动滚动。
3. 每次画面稳定后采集一帧。
4. 拼接器估算相邻帧位移，删除重叠区域并检测固定内容。
5. 达到底部或用户结束后生成预览。
6. 用户裁剪并保存。

验收重点：失败时保留已捕获帧，可重试或降级为手动拼接，不能直接丢失结果。

## 3.6 后台常驻与退出

1. 安装完成后用户启动程序，应用创建命名互斥量并检查是否已有实例。
2. 首个实例初始化 DPI、日志、配置、Slint 事件循环、托盘和全局快捷键，但默认不显示主窗口。
3. 用户按快捷键或点击托盘菜单时，应用按需创建/显示截图、设置、钉图或结果窗口。
4. 用户关闭普通窗口时仅销毁该窗口；主进程继续驻留托盘。
5. 用户从托盘点击“退出”后，应用取消后台任务、关闭钉图、注销快捷键、释放托盘图标并退出。
6. Windows 注销或关机时执行有时限的清理，不阻塞系统关机。
7. 第二次运行安装目录中的程序时，不创建新实例，而是通知已有实例打开设置页或显示托盘提示。

实现约束：

- 使用带当前用户/会话作用域的 Windows 命名互斥量实现单实例判断；使用命名管道向首实例传递版本化启动命令。
- 命名管道 ACL 仅允许当前用户访问；IPC 命令采用白名单，不接收 API Key、任意文件写入或任意命令执行请求。
- 创建一个不可见、无任务栏按钮的轻量 Win32 顶层通知窗口，接收托盘回调、`WM_HOTKEY`、`TaskbarCreated`、`WM_QUERYENDSESSION`、电源/会话和显示器变化通知；不使用 `HWND_MESSAGE`，因为 message-only window 收不到部分系统广播。
- 通知窗口只承担消息分发，不创建渲染表面、不加载截图，也不使用隐藏的大型 Slint 主窗口承担后台驻留。
- 原生消息由应用命令通道转发至 Slint 事件循环，禁止从任意 Win32/IPC 回调线程直接修改 Slint 组件。
- 快捷键注册失败不能导致应用退出，托盘中必须保留重新配置入口。
- “关闭窗口”“隐藏到托盘”“退出进程”是三个独立语义。
- 后台空闲时释放截图帧、标注缓存和无用 GPU 资源；只保留配置、快捷键、托盘与轻量服务。
- 开机启动统一使用当前用户 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`，由应用服务维护；安装器只传递首次安装选择，避免应用和安装器各自维护不同状态。
- Explorer 重启后监听 `TaskbarCreated` 并重建托盘图标；睡眠/唤醒、会话切换和显示器拓扑变化后重新验证快捷键与窗口状态。

验收重点：关闭所有可见窗口后快捷键仍工作；重复启动不产生第二实例；托盘退出后进程、图标、快捷键和后台任务全部清理。

---

## 4. 总体架构

采用“工作区多 crate + Slint 展示层 + 平台适配层 + 事件驱动应用服务”的结构。Slint 已确定为 UI 框架，负责覆盖层、工具栏、标注界面、钉图内容、OCR/翻译结果和设置页；托盘、全局快捷键、单实例、原生窗口样式及捕获能力通过 `windows` crate 封装。业务核心不得依赖 Slint 组件类型，防止 UI 状态渗透到截图与图像处理模块。

```text
┌────────────────────────────────────────────────────┐
│ Slint UI / Application Shell                       │
│ settings, overlay, editor, pin, result panel       │
└───────────────────────┬────────────────────────────┘
                        │ commands / events
┌───────────────────────▼────────────────────────────┐
│ Application Services                               │
│ capture flow, editor flow, longshot, AI tasks      │
└───────┬───────────┬──────────┬───────────┬─────────┘
        │           │          │           │
┌───────▼─────┐ ┌───▼────┐ ┌───▼─────┐ ┌──▼────────┐
│ Capture     │ │ Editor │ │ Imaging │ │ AI Client │
│ Abstraction │ │ Model  │ │ Pipeline│ │ Providers │
└───────┬─────┘ └────────┘ └─────────┘ └───────────┘
        │
┌───────▼────────────────────────────────────────────┐
│ Platform Layer: Windows                            │
│ WGC/DXGI, Win32 windows, DPI, input, clipboard,    │
│ hotkey, credential manager, tray, startup          │
└────────────────────────────────────────────────────┘
```

### 4.1 分层原则

- `domain`：纯数据类型、状态机、坐标系统、编辑命令，不依赖 Win32/UI。
- `application`：编排截图、保存、钉图、AI 请求和长截图任务。
- `platform-windows`：封装 Windows API 和不安全代码。
- `imaging`：像素格式、裁剪、缩放、编码、模糊、拼接。
- `ai`：统一 Provider 接口、OpenAI Responses/Chat Completions 风格适配器、错误标准化。
- `ui-slint`：只负责 Slint 组件、展示状态、输入映射和可见窗口生命周期。
- `app`：负责单实例、托盘常驻、Slint 事件循环、命令分发和进程退出。
- Slint 回调只提交应用命令；不得直接执行捕获、HTTP、编码或长截图计算。

### 4.2 建议工作区结构

```text
snipe/
├─ Cargo.toml
├─ crates/
│  ├─ app/                    # 可执行程序、生命周期、托盘
│  ├─ domain/                 # 公共模型、状态机、几何与命令
│  ├─ capture-core/           # CaptureService 接口、命令与帧模型
│  ├─ capture-windows/        # WGC/DXGI/GDI/窗口枚举/DPI
│  ├─ imaging/                # 裁剪、编码、滤镜、拼接
│  ├─ annotation/             # 标注对象、命令栈、渲染接口
│  ├─ longshot/               # 帧稳定检测、配准与拼接
│  ├─ ai-client/              # Provider、请求、流式输出、重试
│  ├─ secure-storage-windows/ # Windows Credential Manager
│  ├─ config/                 # 非敏感配置、迁移、校验
│  └─ ui-slint/               # Slint 组件、窗口与 Rust 绑定
├─ ui/
│  ├─ overlay.slint           # 截图覆盖层与浮动工具栏
│  ├─ editor.slint            # 标注编辑界面
│  ├─ pin.slint               # 钉图窗口内容
│  ├─ ai-result.slint         # OCR/翻译结果
│  └─ settings.slint          # 设置页
├─ installer/
│  └─ snipe.iss               # Inno Setup 安装脚本
├─ assets/
│  ├─ icons/
│  ├─ cursors/
│  └─ i18n/
├─ tests/
│  ├─ fixtures/               # 非敏感测试图片
│  ├─ golden/                 # 图像黄金样本
│  └─ integration/
├─ scripts/
│  ├─ package.ps1             # release 构建并调用 ISCC.exe
│  ├─ sign.ps1                # 对应用与安装包执行代码签名
│  └─ smoke-test.ps1          # 安装/升级/卸载冒烟测试
└─ docs/
   ├─ architecture.md
   ├─ ui-spec.md
   ├─ ai-provider.md
   └─ release-checklist.md
```

首轮开发可以减少 crate 数量，但模块边界应保持一致；待接口稳定后再拆分，避免一开始过度工程化。

---

## 5. UI 技术方案与验证 PoC

## 5.1 已确定方案

采用 **Slint + Rust + Win32**：

- Slint：声明式界面、截图覆盖层、工具栏、标注编辑器、钉图内容、结果页和设置页。
- Rust 应用层：窗口编排、状态机、图像模型、后台任务和资源生命周期。
- `windows` crate：获取原生窗口句柄，并实现托盘、快捷键、DPI、置顶、透明度、鼠标穿透、单实例和捕获。
- Tokio/reqwest：仅用于网络与异步后台任务，不接管 Slint UI 线程。

不采用 WebView 前端，不引入 Node.js/npm 运行时。业务模型不使用 Slint 类型，以便独立测试，并保留未来替换局部渲染器的能力。

## 5.2 Slint 验证性 PoC

选型已经确定，但正式开发前仍必须以最小纵向切片验证高风险能力：

1. 不显示主窗口，仅托盘后台驻留并响应全局快捷键；
2. 第二次启动时激活已有实例，不重复创建托盘和快捷键；
3. 双显示器混合 DPI 下创建覆盖虚拟桌面的透明层；
4. 显示一张 4K 截图并以目标 60 FPS 更新选区；
5. 创建至少 5 个独立钉图窗口；
6. 通过 HWND 切换置顶、透明度和鼠标穿透；
7. 文本输入支持中英文 IME 和字体回退；
8. 截图窗口关闭后主进程与快捷键继续工作；
9. 连续截图 100 次后检查内存、USER/GDI 对象、句柄与 GPU 资源；
10. 测量 release 构建体积、冷启动、快捷键响应和后台空闲内存。

## 5.3 PoC 通过标准与降级策略

- 核心用例 1～8 必须全部通过，结果写入 `docs/ui-poc-report.md`。
- 若 Slint 的复杂标注画布性能不足，优先保留 Slint 外壳，并为编辑画布增加自定义渲染或 `wgpu`，不更换整个 UI 技术栈。
- 若 Slint 窗口 API 无法设置特殊样式，通过原生 HWND/Win32 平台适配层补足。
- 若单个虚拟桌面覆盖层在混合 DPI 下不稳定，降级为“每显示器一个覆盖窗口”，应用层统一选区坐标。
- M0 结束时冻结 Slint 版本、渲染后端、窗口创建方式和实测性能基线。

---

## 6. 核心技术设计

## 6.1 屏幕捕获

### 后端策略

优先级建议：

1. **Windows Graphics Capture（WGC）**：用于现代 Windows 10/11 的屏幕/窗口捕获；
2. **DXGI Desktop Duplication**：作为低延迟屏幕捕获和部分兼容路径；
3. **GDI BitBlt/PrintWindow**：仅作为兼容回退，不作为主路径。

WGC/DXGI/Direct3D 对象可能具有线程或 apartment 约束，不应强行要求底层对象实现 `Send + Sync`。采用“捕获线程/DispatcherQueue 独占平台对象 + 命令通道 + oneshot 响应”的服务模型。应用层只依赖异步服务接口：

```rust
#[async_trait]
pub trait CaptureService: Send + Sync {
    async fn displays(&self) -> Result<Vec<DisplayInfo>, CaptureError>;
    async fn windows(&self) -> Result<Vec<WindowInfo>, CaptureError>;
    async fn capture(&self, target: CaptureTarget) -> Result<Frame, CaptureError>;
}
```

`CaptureService` 句柄可以跨线程，底层平台后端及 D3D/WGC 资源留在其所有者线程。区域截图优先先捕获目标显示器/桌面帧再以图像本地坐标裁剪，不让覆盖层参与最终帧。多显示器区域由带显示器原点的独立帧组成并在应用层合成，不能假设所有显示器共享相同 DPI、旋转或色彩模式。

### 像素格式

- 内部主格式候选为预乘 Alpha 的 BGRA8；M0 根据 WGC/DXGI、Slint 上传和编码转换成本确认后冻结，禁止模块自行选择格式。
- GPU 捕获后尽量避免多次 CPU/GPU 往返。
- 编码与剪贴板边界才执行必要转换。
- `Frame` 必须携带物理像素尺寸、色彩空间、显示器 ID、时间戳和 stride。

### 自身窗口排除

- 截图前隐藏工具浮层，等待 DWM 完成一帧，或使用窗口捕获排除能力。
- 钉图窗口默认是否参与截图应做成设置项。
- 自动化测试必须验证覆盖层、工具栏不会进入结果图。
- 对 DRM/受保护内容、UAC 安全桌面、独占全屏和高权限窗口明确允许返回“系统限制/权限不足”，不得循环重试或产出误导性空白图。

## 6.2 坐标与 DPI

这是项目最高风险模块之一。统一定义四种坐标，禁止使用裸 `(i32, i32)` 或隐式混用：

- `DesktopPxPoint/DesktopPxRect`：Windows 虚拟桌面的物理像素坐标，允许负数；
- `MonitorPxPoint/MonitorPxRect`：相对某显示器原点的物理像素坐标；
- `ImagePxPoint/ImagePxRect`：相对捕获帧左上角的图像像素坐标；
- `LogicalPoint/LogicalRect`：Slint 窗口内 DIP/逻辑布局坐标。

规则：

- 通过应用 manifest 在任何窗口创建前启用 Per-Monitor DPI Awareness V2。
- 每次窗口跨屏、收到 `WM_DPICHANGED`、显示器插拔或分辨率变化后刷新映射。
- 转换函数必须显式传入显示器/窗口上下文、原点和缩放因子；混合 DPI 下禁止使用一个全局 scale factor。
- 选区持久状态使用 `DesktopPxRect`，裁剪前显式转换为对应帧的 `ImagePxRect`。
- 虚拟桌面左上角不假设为 `(0, 0)`；矩形运算使用有符号坐标并检查溢出。

必须覆盖的测试矩阵：

- 单屏 100%、125%、150%、200%；
- 双屏左右排列且缩放不同；
- 副屏位于主屏左侧或上方；
- 横竖屏混合；
- 运行时切换缩放或插拔显示器。

## 6.3 窗口识别与智能选区

- 使用 `WindowFromPoint`、`EnumWindows`、DWM 扩展边框信息获取候选窗口。
- 过滤不可见窗口、工具自身窗口、透明点击穿透窗口和无有效尺寸窗口。
- 同时保存客户区与扩展窗口边界，以设置控制是否包含阴影。
- 后续可增加 UI Automation 元素级识别，但不作为 MVP 硬依赖。
- 鼠标移动时只更新高亮，不重复执行高成本捕获。

## 6.4 剪贴板与文件保存

- 至少写入标准位图格式；可额外写入 PNG MIME/注册格式以保留 Alpha。
- 文件输出支持 PNG、JPEG、WebP。
- JPEG/WebP 提供质量设置。
- 文件名模板示例：`Screenshot_{yyyy-MM-dd}_{HH-mm-ss}_{w}x{h}`。
- 使用临时文件 + 原子替换降低写入中断风险。
- 保存路径不可用时回退并提示，不静默失败。

## 6.5 标注系统

### 数据模型

标注采用矢量对象保存，最终导出时再与位图合成：

```rust
pub enum Annotation {
    Rect(RectShape),
    Ellipse(EllipseShape),
    Line(LineShape),
    Arrow(ArrowShape),
    Freehand(PathShape),
    Highlighter(PathShape),
    Text(TextShape),
    Mosaic(MosaicShape),
    Blur(BlurShape),
    Step(StepShape),
}
```

所有对象包含：ID、几何数据、样式、层级、可见性；坐标基于原图物理像素或归一化画布坐标，并在全项目固定一种方案。

### 撤销/重做

采用命令模式：

- `AddAnnotation`
- `RemoveAnnotation`
- `UpdateGeometry`
- `UpdateStyle`
- `ReorderAnnotation`
- `CropCanvas`

拖动过程中只更新预览；鼠标释放后合并成一个历史命令，避免命令栈爆炸。

### 渲染

- 编辑态：GPU/框架画布渲染矢量覆盖层。
- 导出态：离屏以原图分辨率渲染，不能按窗口缩放后的尺寸导出。
- 马赛克、模糊只对原图对应区域处理，避免缩放引发边界偏差。
- 文本必须处理字体回退、中文、Emoji 与高 DPI。

## 6.6 钉图窗口

`PinManager` 管理多个 `PinWindow`：

- 每个钉图拥有稳定 ID、图像引用、位置、大小、缩放、透明度、置顶、穿透状态。
- 图片数据通过 `Arc` 共享，避免菜单操作和窗口显示产生多份 4K 位图。
- 缩放采用预览纹理缓存，保存与 OCR 始终使用原图。
- 鼠标穿透启用后，必须提供全局快捷键或托盘菜单恢复操作。
- 关闭窗口时及时释放 GPU 纹理与大图缓存。

可选持久化字段只保存元数据和用户明确允许保留的图片路径；默认不自动落盘敏感截图。

## 6.7 颜色拾取

- 取色使用捕获帧缓存，而不是每次鼠标移动调用昂贵的整屏 API。
- 放大镜显示鼠标附近像素网格与中心十字。
- 格式化层独立实现，支持：
  - `#RRGGBB`
  - `#RRGGBBAA`
  - `rgb(r, g, b)`
  - `hsl(h, s%, l%)`
  - `hsv(h, s%, v%)`
- 明确色彩管理限制：MVP 先保证 SDR/sRGB；HDR 与广色域在兼容性阶段专项验证。

## 6.8 长截图

### MVP 策略

采用“固定区域连续采帧 + 图像内容配准”，避免把首版完全绑定到特定应用的滚动 API。

流水线：

1. 用户指定捕获区域；
2. 记录首帧；
3. 检测滚动动作或主动发送滚轮事件；
4. 等待画面稳定；
5. 捕获下一帧；
6. 在预期重叠带内估计垂直位移；
7. 计算置信度并拼接；
8. 判断是否到达末尾；
9. 输出完整图与诊断信息。

### 配准算法迭代

第一版优先使用成本可控的算法：

- 图像转灰度并适度降采样；
- 在中间内容区域提取若干水平条带；
- 使用归一化互相关、绝对差或特征点方法搜索重叠偏移；
- 多条带投票得出位移；
- 对固定头部/悬浮侧栏做掩膜或裁除；
- 置信度不足时暂停并要求用户调整。

后续再评估 OpenCV 绑定。若引入 OpenCV 会增加包体、构建和许可证审查成本，因此应先用纯 Rust 图像算法完成基准验证。

### 稳定检测

连续采样同一区域的缩略图；当帧差低于阈值并持续若干帧后认为页面稳定。对视频、光标闪烁、动画广告采用区域鲁棒统计，避免永远无法稳定。

### 终点检测

满足任一条件停止：

- 新帧与上一帧位移接近零且内容高度重复；
- 连续多次捕获无新增区域；
- 用户手动结束；
- 达到最大高度、帧数、内存或时长上限。

### 内存控制

- 不长期保留所有原始全尺寸帧；确认拼接后释放无用帧。
- 超长图片按条带/分块构建。
- 设置最大像素数和预计内存提示。
- 编码器支持流式写入时优先流式输出。

### 自动滚动

分两级实现：

1. 通用输入模拟：向目标窗口发送滚轮/按键，易实现但可靠性有限；
2. UI Automation/控件接口：识别可滚动对象并设置滚动位置，兼容性更复杂。

首版必须保留手动滚动模式作为稳定降级路径。

## 6.9 OCR 与翻译服务

### Provider 抽象

```rust
#[async_trait]
pub trait VisionProvider: Send + Sync {
    async fn recognize(
        &self,
        image: EncodedImage,
        options: OcrOptions,
        cancel: CancellationToken,
    ) -> Result<OcrResult, AiError>;

    async fn translate_image(
        &self,
        image: EncodedImage,
        options: TranslationOptions,
        cancel: CancellationToken,
    ) -> Result<TranslationResult, AiError>;
}
```

### OpenAI-compatible 配置

非敏感配置可包含：

```toml
[ai]
provider = "openai-compatible"
base_url = "https://api.openai.com/v1"
model = "<user-configured-vision-model>"
timeout_seconds = 60
max_retries = 2

[translation]
target_language = "zh-CN"
preserve_paragraphs = true
```

API Key 只存储在 Windows Credential Manager，配置文件中最多保存凭据条目的引用名。

### 请求设计

- 优先使用服务商支持的图像输入消息格式。
- 对过大图片先按最长边、文件大小阈值压缩；提示这可能影响小字识别。
- OCR 提示词要求只输出识别文本，避免模型添加解释。
- 翻译提示词要求保留段落、数字、专有名词和代码片段。
- 对长截图采用分块识别，并保留块顺序；块间使用少量重叠避免断句，并去除合并后的重复段落。
- 首版明确实现两个适配器：OpenAI 官方 Responses API，以及 OpenAI Chat Completions 风格的多模态兼容 API；“兼容”只承诺文档中列出的请求字段、Bearer 鉴权和响应结构，不承诺所有第三方实现。
- 模型输出一律作为不可信纯文本处理：不执行其中的指令、脚本或链接，不直接渲染未净化 HTML/Markdown。
- OCR 属于概率性识别；结果页提示用户校对数字、金额、日期和敏感业务文本，不把模型结果宣称为确定事实。

### 推荐结果结构

```rust
pub struct OcrResult {
    pub text: String,
    pub language: Option<String>,
    pub blocks: Vec<TextBlock>,
    pub provider: String,
    pub model: String,
}

pub struct TranslationResult {
    pub source_text: Option<String>,
    pub translated_text: String,
    pub source_language: Option<String>,
    pub target_language: String,
}
```

多模态大模型不一定返回可靠文本框坐标，因此 MVP 的 `blocks` 可以为空；若未来接入专用 OCR 服务，再提供坐标。

### 错误分类

统一映射：

- 缺少 API Key；
- URL/证书错误；
- 401/403 鉴权失败；
- 429 限流/额度不足；
- 模型不支持图片；
- 图片过大；
- 超时；
- 用户取消；
- 响应格式不兼容；
- 服务端错误。

UI 不直接展示原始响应中的敏感数据；调试日志也必须脱敏。

---

## 7. 状态机与并发模型

## 7.1 应用状态

```text
Idle
 ├─> Capturing
 │    ├─> Selecting
 │    ├─> Editing
 │    └─> Idle
 ├─> LongShotCapturing
 │    ├─> LongShotPreview
 │    └─> Idle
 └─> ColorPicking
      └─> Idle
```

- 同一时间只允许一个全局捕获会话。
- 钉图窗口和 AI 后台任务可并行存在。
- 新截图请求到达时，如果已有捕获会话，应播放提示而不是重复创建覆盖层。

## 7.2 异步任务

- UI 主线程：Slint 事件循环、可见窗口、输入和轻量状态更新。
- 原生消息入口：隐藏的轻量 Win32 顶层通知窗口处理托盘、`WM_HOTKEY`、Explorer 重启及电源/会话/显示器通知，并通过命令通道投递到应用/UI 层；命名管道监听器运行在后台任务中。
- M0 必须验证所选 Slint 后端与原生通知窗口消息泵能够共存，且在没有可见 Slint 窗口时事件循环不会退出；若不能，则把原生窗口放入专用 STA 线程并只通过通道通信，并采用 Slint 支持的事件循环保持方案。不得靠常驻隐藏渲染窗口维持进程。
- 异步运行时：HTTP、文件 I/O、任务取消与超时。
- 图像工作线程池：编码、模糊、拼接等 CPU 密集任务。
- GPU/Windows Capture 对象严格遵守其线程与 apartment 约束。

所有长任务必须支持：进度、取消、超时、窗口销毁后的安全回收。

## 7.3 事件示例

```rust
pub enum AppEvent {
    HotkeyPressed(HotkeyAction),
    CaptureStarted(CaptureSessionId),
    SelectionChanged(DesktopPxRect),
    CaptureCompleted(Frame),
    AnnotationChanged,
    PinRequested(ImageId),
    AiProgress(TaskId, AiProgress),
    AiCompleted(TaskId, AiResult),
    TaskFailed(TaskId, UserFacingError),
}
```

---

## 8. 配置、隐私与安全

## 8.1 配置分类

普通配置默认放在 `%LOCALAPPDATA%\Snipe\config`，缓存与日志分别放在 `%LOCALAPPDATA%\Snipe\cache` 和 `%LOCALAPPDATA%\Snipe\logs`；用户主动保存的截图默认放入系统“图片”目录下的 `Snipe` 子目录。所有路径通过 Windows Known Folder API 获取，不硬编码用户名或盘符。

普通配置文件可保存：

- 快捷键；
- 默认保存目录与格式；
- 文件命名模板；
- UI 语言、主题；
- AI Base URL、模型名、超时；
- 是否显示鼠标、是否包含窗口阴影；
- 长截图限制；
- 最近使用的标注样式。

不得保存在普通配置或日志中：

- API Key；
- Authorization 请求头；
- 用户截图内容；
- OCR/翻译全文；
- 未经用户允许的截图历史。

## 8.2 API Key 管理

- 使用 Windows Credential Manager 保存。
- 设置页仅显示“已配置/未配置”和掩码尾部。
- 提供“更新”“删除”“测试连接”。
- 测试连接不得把 Key 写入命令行参数。
- 崩溃报告、日志和错误 UI 中统一执行密钥脱敏。

## 8.3 隐私提示

- 第一次使用在线 OCR/翻译时说明图片会发送给所选服务商。
- 请求按钮附近显示当前 Provider 和域名。
- 对自定义 Base URL 标识“第三方服务”。
- 提供发送前压缩与图片预览。
- 默认不记录请求正文和响应正文。

## 8.4 网络安全

- 默认仅允许 HTTPS；HTTP 仅允许用户显式确认后用于 localhost/局域网测试，正式发布配置不默认开启。
- 设置独立的连接、首字节和总请求超时，并限制响应体大小。
- 携带 Authorization 时默认禁用 HTTP 重定向；任何情况下都不得把凭据转发到不同 origin。
- API Key 凭据条目绑定规范化的 provider + origin；修改 Base URL 后必须重新确认或选择凭据。
- 对 URL 做校验，不把 API Key 拼进 URL；代理配置不得进入日志。
- 依赖定期执行许可证与漏洞审查。

---

## 9. 依赖候选清单

最终版本号在创建工程时锁定，并通过 `Cargo.lock` 固化。

| 领域 | 候选依赖/技术 |
|---|---|
| Windows API | `windows` crate |
| Windows EXE 资源 | `winresource` 或等价构建方案，用于 manifest、图标和版本信息 |
| Rust 构建目标 | `x86_64-pc-windows-msvc`，固定 Rust toolchain 与 Windows SDK 基线 |
| 异步运行时 | `tokio` |
| HTTP | `reqwest` + rustls（优先避免额外原生 TLS 部署复杂度） |
| 序列化 | `serde`, `serde_json`, `toml` |
| 错误 | `thiserror`; 应用边界可使用 `anyhow` |
| 日志 | `tracing`, `tracing-subscriber` |
| 图像 | `image`; WebP/高级编码能力另行验证 |
| ID | `uuid` 或轻量自增 ID |
| 并发取消 | `tokio-util::sync::CancellationToken` |
| 密钥存储 | Win32 Credential API 封装，或经审查的 `keyring` |
| UI | `slint`, `slint-build`，版本在 M0 后锁定 |
| Windows 集成 | `windows` crate；托盘库仅在验证后采用 |
| 安装包 | Inno Setup 6，使用 `ISCC.exe` 编译 `installer/snipe.iss` |
| 测试 | Rust 内置测试、`proptest`、快照/黄金图片工具 |

依赖准入要求：

- M0 必须确定 Slint 的合规许可路径（GPLv3、Royalty-free 或商业许可之一），并记录应用分发模式与义务；未完成许可证审查不得发布二进制。
- 检查所有直接/传递依赖及 Inno Setup 的许可证、活跃度与安全公告；
- 避免仅为一个简单函数引入大型依赖；
- Windows 不安全代码集中封装；
- 不引入会将用户图片自动上传的 SDK；
- OpenCV 仅在纯 Rust 长截图算法无法达到验收标准时考虑。

---

## 10. 里程碑与任务拆解

以下以 1 名熟悉 Rust/Windows 桌面开发的全职开发者估算约 22～30 周（含兼容性测试与 Beta 缓冲）；18～24 周只能作为功能原型目标，不应承诺为稳定正式版日期。多人并行可缩短部分模块，但 Windows 捕获、长截图、UI 联调和发布签名仍存在关键路径。

## M0：需求冻结与技术验证（第 1～2 周）

### 任务

- [ ] 建立 Cargo workspace、格式化、lint、测试和 CI 骨架。
- [ ] 完成 Slint 及全部分发依赖的许可证审查，形成 ADR；闭源/商业分发前确定可用许可。
- [ ] 建立 Slint 验证性 PoC，完成后台驻留、原生通知窗口消息泵、透明覆盖层、多钉图和 HWND/Win32 集成。
- [ ] 测量 Slint release 包体、冷启动、空闲内存、4K 画布帧率和长期资源变化。
- [ ] 验证 WGC、DXGI、GDI 三种捕获路径，以及黑帧、受保护内容、UAC 安全桌面和远程桌面的失败表现。
- [ ] 确定最低 Windows build，并把运行时能力探测与回退矩阵写入 ADR。
- [ ] 验证 Windows 多屏/DPI 坐标映射。
- [ ] 验证全局快捷键、托盘、置顶、透明和鼠标穿透。
- [ ] 验证 Windows Credential Manager 读写与删除。
- [ ] 通过最小客户端调用 OpenAI-compatible 视觉接口。
- [ ] 输出 ADR（Architecture Decision Record）。

### 交付物

- `docs/ui-poc-report.md`
- `docs/adr/0001-slint-ui-license-and-window-integration.md`
- `docs/adr/0002-capture-backend-and-minimum-windows-build.md`
- `installer/snipe.iss` 最小安装脚本
- 可运行的技术验证程序

### 验收标准

- 在至少两台 Windows 设备或两个不同 DPI 环境中运行。
- 能完成“后台驻留 → 快捷键 → 捕获 → Slint 选区 → 复制 → 关闭窗口后继续驻留”的最小链路。
- Slint 高风险用例全部通过，并确定渲染后端、原生窗口集成方式、首选捕获后端和回退策略。
- Inno Setup 可生成能正常安装、启动和卸载的测试版 `setup.exe`。

## M1：应用骨架与普通截图（第 3～6 周）

### 任务

- [ ] 使用包含当前用户 SID/会话标识的本地命名互斥量实现应用单实例，避免不同登录用户相互阻塞。
- [ ] 使用受当前用户 ACL 保护的命名管道，把二次启动命令转发给已有实例。
- [ ] 实现轻量原生通知窗口、托盘生命周期、Explorer 重启恢复、菜单命令和显式退出流程。
- [ ] 配置加载、校验、版本迁移框架。
- [ ] 全局快捷键注册、注销与冲突提示。
- [ ] 实现当前用户级开机启动设置。
- [ ] 处理 Explorer 重启、睡眠/恢复、会话切换和显示器热插拔。
- [ ] 显示器枚举与虚拟桌面模型。
- [ ] 实现异步 `CaptureService` 及其线程隔离的 Windows 后端。
- [ ] 截图覆盖层、暗色遮罩和十字光标。
- [ ] 拖拽选区、调整手柄、尺寸提示。
- [ ] 窗口识别与高亮。
- [ ] 剪贴板和 PNG/JPEG/WebP 保存。
- [ ] 基础设置页。

### 验收标准

- 覆盖 DPI 测试矩阵。
- 全屏、窗口、区域截图均可复制与保存。
- 连续截图 100 次无崩溃、无明显资源持续增长。
- 关闭设置、截图和结果窗口后，进程留在托盘且快捷键继续工作。
- 重复启动不会创建第二个托盘图标，并能通知已有实例。
- 托盘“退出”会注销快捷键、移除图标、取消任务并结束进程。
- 快捷键冲突、保存失败、捕获失败均有明确提示。

## M2：标注编辑器（第 7～10 周）

### 任务

- [ ] 标注对象模型和序列化模型。
- [ ] 选择、移动、缩放、删除。
- [ ] 矩形、椭圆、线、箭头、画笔、马克笔。
- [ ] 文本输入与字体回退。
- [ ] 马赛克、模糊、序号标记。
- [ ] 样式工具栏。
- [ ] 撤销/重做命令栈。
- [ ] 裁剪。
- [ ] 离屏高分辨率导出。

### 验收标准

- 常见操作全部支持撤销/重做。
- 4K 图片上编辑流畅，拖动过程目标 60 FPS。
- 缩放预览不会改变最终导出清晰度。
- 中英文、数字和常见 Emoji 文本可正常导出。

## M3：钉图与取色（第 11～13 周）

### 任务

- [ ] `PinManager` 与多窗口生命周期。
- [ ] 拖动、缩放、透明度、置顶、穿透。
- [ ] 钉图右键菜单和快捷键。
- [ ] 取色模式与放大镜。
- [ ] HEX/RGB/HSL/HSV 转换与复制。
- [ ] 大图纹理缓存与释放策略。

### 验收标准

- 同时打开 10 个普通尺寸钉图窗口仍可正常操作。
- 穿透开启后可通过快捷键或托盘可靠恢复。
- 取色与实际截图像素一致；DPI/负坐标屏幕无偏移。
- 关闭钉图后内存可回落到合理范围。

## M4：AI OCR 与翻译（第 14～16 周）

### 任务

- [ ] 实现 `VisionProvider`、OpenAI Responses 与 Chat Completions 风格多模态适配器。
- [ ] 设置页：Base URL、模型、Key、超时、测试连接。
- [ ] Credential Manager 安全存取。
- [ ] 图片压缩、编码和请求大小限制。
- [ ] OCR 提示词模板与结果面板。
- [ ] 翻译目标语言与结果面板。
- [ ] 长图分块请求与结果合并。
- [ ] 任务进度、取消、重试和错误分类。
- [ ] 日志脱敏测试。

### 验收标准

- OpenAI 官方 Responses API 和至少一个已列名的 Chat Completions-compatible 多模态服务通过契约测试。
- 文档准确列出已验证服务、API 风格和不兼容项，禁止笼统承诺全部 OpenAI-compatible 服务。
- 无 Key、错误 Key、错误模型、429、超时、断网、超大响应和跨域重定向均正确处理。
- UI 在请求期间保持响应，可取消。
- 配置文件、日志、崩溃信息中均找不到完整 API Key。

## M5：长截图（第 17～22 周）

### 任务

- [ ] 长截图区域选择与会话状态机。
- [ ] 手动滚动采集。
- [ ] 帧差与稳定检测。
- [ ] 重叠搜索、位移估计与置信度。
- [ ] 固定区域检测与裁除。
- [ ] 流式/分块拼接与内存上限。
- [ ] 终点检测。
- [ ] 拼接预览与最终裁剪。
- [ ] 自动滚轮模式。
- [ ] 失败诊断与恢复已捕获结果。
- [ ] 建立长截图测试样本集。

### 验收标准

- 至少 30 个冻结样本覆盖 Chromium 静态页面、固定页头、重复纹理、Win32 长列表和文档窗口。
- 按 1.3 定义的成功标准统计结果，不以主观“可用”代替指标。
- 默认安全上限在 M0/M5 基准后冻结；初始建议为 50,000 px 高、100 MP、512 MiB 工作集增量、300 帧或 5 分钟，任一达到即停止并保留结果。启动前根据可用内存预估，低内存环境使用更低动态上限。
- 拼接失败不会丢弃已经采集的帧。
- 达到资源上限时安全终止并解释原因。
- 固定页头场景不会大面积重复。

## M6：稳定性、可访问性与发布（第 23～27 周）

### 任务

- [ ] 全功能回归测试。
- [ ] 键盘导航、焦点与屏幕阅读器基础支持。
- [ ] 多语言资源框架，至少中英文。
- [ ] 高对比度、深浅主题验证。
- [ ] 性能剖析和资源泄漏检查。
- [ ] 崩溃恢复与本地诊断包。
- [ ] 编写并固化 `installer/snipe.iss`，生成单文件 `.exe` 安装包。
- [ ] 实现安装、覆盖升级、卸载、可选开机启动和安装后运行。
- [ ] 为应用 EXE、必要 DLL 和最终安装 EXE 建立代码签名流程。
- [ ] 自动更新方案评估；若首版不做，提供手动检查更新。
- [ ] 隐私说明、第三方许可证、用户文档。

### 验收标准

- P0/P1 缺陷清零。
- 普通用户无需管理员权限即可完成默认安装、覆盖升级和卸载。
- 安装、升级、卸载不遗留异常进程、托盘图标或开机启动项；是否删除用户配置与凭据由卸载选项决定。
- 安装包为单个 `.exe`，文件属性中的产品名、公司、版本、图标和签名正确。
- SmartScreen/签名流程验证完成。
- 发布清单全部通过。

## M7：v1.0 候选版本（第 28～30 周，Beta 与缓冲）

- [ ] 小范围 Beta。
- [ ] 收集兼容性问题，不收集截图内容。
- [ ] 修复支持范围内的 GPU、远程桌面和混合 DPI 问题；验证 HDR，并对未支持场景给出明确限制。
- [ ] 冻结依赖与构建环境。
- [ ] 生成正式安装包、校验值和版本说明。

---

## 11. 测试策略

## 11.1 单元测试

重点覆盖：

- 坐标转换和矩形裁剪；
- 颜色空间转换与格式化；
- 文件命名模板；
- 标注命令栈；
- AI 错误映射与响应解析；
- 长截图位移计算、终点检测；
- 配置迁移与校验。

使用属性测试验证：

- `DesktopPx/MonitorPx/ImagePx/Logical` 的合法转换在明确上下文和允许误差内可逆；
- 裁剪区域始终位于图像边界内；
- 颜色转换输出在合法范围；
- 任意撤销/重做序列不破坏模型状态。

## 11.2 黄金图片测试

为以下渲染建立固定输入和期望输出：

- 箭头、圆角矩形、文本、马赛克、模糊；
- 不同缩放比例导出；
- 长截图重叠与固定头部；
- Alpha 混合与透明背景。

比较时允许小范围抗锯齿误差，但必须设置像素差比例阈值。

## 11.3 集成测试

- 捕获 → 裁剪 → 标注 → 编码 → 读取验证。
- 截图 → 剪贴板 → 外部应用粘贴人工/自动验证。
- 钉图创建/关闭后的资源回收。
- AI mock server：成功、流式、超时、429、500、非法 JSON、超大响应、同源/跨域重定向、取消。
- Credential Manager 创建、读取、更新、删除，以及更改 Base URL 后不误用旧凭据。
- 单实例 IPC：同用户命令成功、非法命令拒绝、其他用户/低权限边界验证。
- Explorer 重启后托盘恢复；睡眠/唤醒和显示器热插拔后快捷键及坐标恢复。
- Inno Setup：全新安装、运行中升级、降级拒绝、保留数据卸载和完全清理卸载。

## 11.4 手工兼容矩阵

### 操作系统

- Windows 10 22H2。
- Windows 11 当前受支持版本。
- 本地桌面与远程桌面。

### 显示环境

- Intel、NVIDIA、AMD GPU 各至少一组。
- 单屏、双屏、三屏。
- 混合 DPI、横竖屏、负坐标。
- SDR；HDR 作为重点兼容项记录限制。

### 应用场景

- Chromium/Edge/Chrome。
- Electron 应用。
- Windows 资源管理器。
- Office 类文档窗口。
- 高权限窗口。
- 全屏应用与 UAC 安全桌面（明确系统限制）。

## 11.5 性能测试

记录 P50/P95：

- 快捷键到覆盖层可交互时间；
- 1080p/4K/多屏捕获耗时；
- PNG/JPEG/WebP 编码耗时；
- 标注画布帧率；
- 10 个钉图窗口内存；
- 50,000 px 长图拼接时间和峰值内存。

每次发布保存基准结果，超过既定阈值时阻止发布或明确批准回归。

---

## 12. CI/CD 与工程规范

## 12.1 持续集成

每次提交执行：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --release --locked --target x86_64-pc-windows-msvc
```

补充任务：

- 使用 `cargo audit`/`cargo deny` 或等价工具执行依赖漏洞、来源与许可证扫描；
- Windows release 构建、EXE manifest/版本资源检查；
- 黄金图片测试；
- 安装包 smoke test；
- 产物哈希生成。
- 签名证书只允许受保护的正式发布任务访问；PR、普通分支和 fork 构建不得获得签名密钥。

## 12.2 代码规范

- 应用 EXE 使用 Windows GUI subsystem，正式构建不得弹出控制台窗口；CLI/诊断工具单独建 binary。
- `unsafe` 仅放在平台适配边界，并写明安全前提。
- 业务错误使用结构化 enum，不使用字符串判断。
- UI 不直接调用裸 Win32；通过平台服务接口访问。
- 公共 API 需要 rustdoc。
- 复杂状态机和坐标转换必须有测试。
- 不在日志中输出图片 Base64、完整请求、响应或 API Key。
- CPU 密集工作不得在 UI 线程执行。

## 12.3 分支与版本

- 主分支保持可构建。
- 功能分支按里程碑拆分，避免一个分支同时实现多个核心模块。
- 使用语义化版本。
- 用户配置包含 schema version，并提供向前迁移。

---

## 13. 发布与安装

### 13.1 固定交付形式

首版只提供 **Windows x64 的 Inno Setup 单文件 `.exe` 安装包**，例如：

```text
Snipe-Setup-0.1.0-x64.exe
Snipe-Setup-0.1.0-x64.exe.sha256
```

同时发布版本说明和第三方许可证清单。首版不提供 ZIP 便携版、MSIX 或 Microsoft Store 包。ARM64 仅保持架构可扩展性，不作为首版正式产物。

安装程序和主程序都应使用统一产品图标、版本资源和合法的 Windows 代码签名。用户从浏览器下载后直接运行 `Snipe-Setup-<version>-x64.exe` 即可安装。

### 13.2 Inno Setup 工程

- 安装脚本固定为 `installer/snipe.iss`。
- `scripts/package.ps1` 先执行 Rust release 构建与资源检查，再调用签名脚本签署主程序，随后调用 Inno Setup 6 的 `ISCC.exe` 并签署最终安装包。
- 版本号由 Cargo workspace 的唯一版本源注入，禁止 Cargo、EXE 资源和 `.iss` 各自维护不同版本。
- 安装包应包含应用 EXE、Slint 所需运行资源、图标、许可证和必要文档；优先静态或随包部署依赖，不能依赖开发机环境。
- 正式构建按“主程序签名并加 RFC 3161 时间戳 → 编译安装包 → 安装包签名并加时间戳 → SHA-256”顺序执行。
- 构建脚本检查未签名产物、缺失资源和版本不一致并立即失败。

`.iss` 的基线要求如下，实际值由构建脚本注入：

```ini
[Setup]
AppId={{<固定-GUID>}
AppName=Snipe
AppVersion=<Cargo-Version>
DefaultDirName={localappdata}\Programs\Snipe
PrivilegesRequired=lowest
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
OutputBaseFilename=Snipe-Setup-<Version>-x64
UninstallDisplayIcon={app}\snipe.exe
CloseApplications=yes
RestartApplications=no
```

实现时还需要配置 `[Files]`、`[Icons]`、`[Tasks]`、`[Run]`、`[UninstallDelete]` 和 Pascal Script 生命周期钩子。`AppId` 一经首个公开版本发布不得更改，否则会破坏覆盖升级识别。不得把 API Key、签名私钥或证书密码写进 `.iss`。

安装器不配置与应用互斥量相同的 `AppMutex`：该指令可能在自定义 IPC 执行前直接阻止安装。升级/卸载脚本应先运行现有 `snipe.exe --shutdown-for-update`；该临时进程通过单实例 IPC 请求首实例检查活动任务并优雅退出，安装器等待有界时间后再进入文件替换。`CloseApplications=yes` 仅作为文件占用的第二层保护，超时后提示用户手动关闭，不默认强杀。

### 13.3 默认安装行为

- 使用当前用户级安装，默认不触发 UAC；默认目录位于 `{localappdata}\Programs\Snipe`。
- 在开始菜单创建“Snipe”和“卸载 Snipe”入口；桌面快捷方式作为可选任务，默认不创建。
- 安装完成页提供“立即运行 Snipe”，启动后不弹出大型主窗口，仅进入托盘并可显示首次使用引导。
- 提供“随 Windows 启动”可选项；仅全新安装时，安装器将选择通过首次启动参数传给应用，由应用统一写入 HKCU Run。覆盖升级不得覆盖用户现有选择，安装后仍可在设置页修改。
- 注册标准卸载信息、DisplayVersion、Publisher、URL 和图标。
- 不擅自覆盖系统截图快捷键；首次启动注册失败时引导用户更换。
- 安装路径与卸载命令正确引用，避免空格或非 ASCII 路径问题。

### 13.4 升级与运行中进程

- 相同 AppId 用于所有版本，支持直接运行新安装 EXE 覆盖升级。
- 升级前调用已安装程序的 `--shutdown-for-update`，通过版本化 IPC 请求首实例优雅退出；若存在未保存编辑、长截图或 AI 任务，应由应用提示并允许取消升级。
- IPC 返回明确状态：`ready`、`busy`、`cancelled`、`incompatible`；Inno Pascal Script 根据状态继续、等待或中止，不能忽略用户取消。
- 安装器等待进程退出并再次检查文件占用；失败时提示用户关闭，不静默强杀正在处理截图的进程。
- 升级保留普通配置、用户截图和 Credential Manager 中的 Key。
- 安装失败或取消时不得破坏当前可运行版本。
- 降级安装默认阻止，除非显式使用维护参数并提示配置兼容风险。

### 13.5 卸载

- 卸载前通过 `--shutdown-for-update` 同一 IPC 协议请求后台进程退出，并确认托盘图标、快捷键与开机启动项已清理。
- 默认删除程序文件，但保留用户截图。
- 卸载页询问是否删除应用配置、缓存、日志和 Windows Credential Manager 中的 API Key；凭据清理必须在应用 EXE 被删除前执行，并验证返回状态。
- 用户选择保留配置时，不保留临时截图或可安全重建的缓存。
- 卸载结束后验证安装目录中无被占用的程序文件；必要时安排重启后删除并明确提示。

### 13.6 安装包测试

每个候选版本至少验证：

1. Windows 10/11 普通用户全新安装；
2. 包含空格和中文用户名环境；
3. 安装后立即启动并显示一个托盘图标；
4. 快捷键截图与钉图；
5. 旧版本覆盖升级并保留配置/Key；
6. 程序运行时升级的退出协调；
7. 保留数据卸载和完全清理卸载；
8. 安装包与主程序签名、版本资源和 SHA-256；
9. 安装、升级、卸载后开机启动项状态正确。

### 13.7 更新

首版可仅在设置页提供手动“检查更新”入口，下载新的签名 `.exe` 安装包并由用户确认运行。若后续实现自动更新：

- 更新清单必须签名并通过 HTTPS 获取；
- 校验下载文件的签名与 SHA-256；
- 支持延后更新；
- 更新失败不破坏当前可运行版本；
- 更新器复用 Inno Setup 覆盖升级路径，不维护第二套安装逻辑。

---

## 14. 风险清单与应对

| 风险 | 概率 | 影响 | 应对措施 |
|---|---:|---:|---|
| 混合 DPI 导致选区偏移 | 高 | 高 | 坐标类型强约束、Per-Monitor V2、完整矩阵测试 |
| WGC/DXGI 在不同 GPU/远程桌面异常 | 中 | 高 | 多后端回退、启动能力探测、诊断日志 |
| Slint 特殊窗口或画布能力不足 | 中 | 高 | M0 验证性 PoC；特殊样式走 HWND/Win32，必要时仅替换画布渲染器 |
| 长截图在动态页面拼接失败 | 高 | 高 | 手动模式、置信度提示、固定区域检测、保留原始帧 |
| 大图造成内存峰值或崩溃 | 中 | 高 | 分块拼接、资源上限、预估提示、及时释放 |
| OCR 模型/API 格式差异 | 高 | 中 | Provider 适配层、自定义模型/Base URL、mock 测试 |
| API Key 泄漏 | 低 | 极高 | Credential Manager、日志脱敏、网络请求审查 |
| 在线 OCR 上传敏感截图 | 中 | 高 | 首次提示、显示域名、默认不记录、用户主动触发 |
| HDR/广色域颜色不准确 | 中 | 中 | MVP 标注 SDR 限制，建立专项色彩管理计划 |
| 杀毒软件/SmartScreen 误报 | 中 | 中 | 签名主程序与 Inno Setup 安装包、稳定发布域名、公开 SHA-256 |
| 后台常驻资源缓慢泄漏 | 中 | 高 | 72 小时 soak test、句柄/GDI/GPU 指标、按需创建和销毁大资源 |
| 升级时后台进程占用文件 | 高 | 中 | IPC 优雅退出、Inno Setup 进程检测、禁止静默强杀用户任务 |
| 高权限/受保护窗口无法捕获或注入滚动 | 中 | 中 | 遵守 UIPI/安全桌面边界，不默认提权；清晰提示并降级为手动流程 |
| 标注文本在不同机器字体不一致 | 中 | 中 | 字体回退、导出前路径/字形渲染测试 |

---

## 15. 可观测性与诊断

- 使用结构化本地日志，正式版默认级别为 `info`，不启用包含请求体、响应体、图片内容或凭据的 debug/trace 记录。
- 日志滚动和大小限制，默认保留较短周期。
- 为一次截图、长截图、AI 请求生成随机任务 ID，不使用图片内容作为标识。
- 记录：耗时、像素尺寸、后端名称、错误分类、重试次数。
- 不记录：图片、OCR/翻译正文、Authorization、完整 API Key。
- 提供“导出诊断包”，导出前列出内容，并再次执行脱敏。
- 遥测默认不启用；若未来加入，必须主动征得同意且不得采集截图内容。

---

## 16. 功能验收清单

## 16.1 常规截图

- [ ] 快捷键可配置并检测冲突。
- [ ] 区域、窗口、全屏截图可用。
- [ ] 多显示器和不同 DPI 坐标准确。
- [ ] 覆盖层与工具栏不进入截图。
- [ ] 复制、保存、取消行为正确。
- [ ] 文件格式和命名模板正确。

## 16.2 标注

- [ ] 所有 MVP 工具可用。
- [ ] 撤销/重做覆盖所有编辑操作。
- [ ] 文本支持中文 IME。
- [ ] 马赛克/模糊导出正确。
- [ ] 原分辨率导出无明显失真。

## 16.3 钉图

- [ ] 多钉图窗口互不影响。
- [ ] 缩放、透明度、置顶、穿透可用。
- [ ] 穿透可可靠解除。
- [ ] 原图用于保存/OCR，不使用低清预览。

## 16.4 OCR/翻译

- [ ] OpenAI 和自定义兼容地址可配置。
- [ ] API Key 安全存储。
- [ ] 可选择模型和目标语言。
- [ ] 请求可取消、超时、重试。
- [ ] 常见错误转换为用户可理解提示。
- [ ] 日志无敏感内容。

## 16.5 长截图

- [ ] 手动滚动模式可用。
- [ ] 自动滚动有降级方案。
- [ ] 能检测稳定、重叠和终点。
- [ ] 固定头部场景基本可用。
- [ ] 失败时可保留已捕获内容。
- [ ] 大图受到资源上限保护。

## 16.6 颜色拾取

- [ ] 放大镜显示正确。
- [ ] HEX/RGB/HSL/HSV 正确转换。
- [ ] 点击与快捷键复制可用。
- [ ] 多屏/DPI 下取色位置准确。

---

## 17. Definition of Done 与发布门禁

缺陷等级：P0 = 数据泄露、密钥泄露、安装/卸载破坏系统或普遍崩溃；P1 = 核心截图链路不可用、严重错图/数据丢失、无法退出/升级；P2 = 有明确绕行方案的功能或兼容性问题。

一个功能只有在满足以下全部条件后才算完成：

1. 产品行为与验收条件明确；
2. 正常路径和关键失败路径均已实现；
3. 单元/集成测试通过；
4. 不阻塞 UI 线程；
5. 多 DPI 场景完成验证（若涉及窗口、坐标或图像）；
6. 日志不包含密钥或用户图片内容；
7. 用户可见文本进入 i18n 资源；
8. 文档与配置示例已更新；
9. `fmt`、`clippy`、测试、release 构建通过；
10. 无未处理的 P0/P1 缺陷。

v1.0 发布还必须同时满足：

- M0～M7 的验收标准全部完成并有测试证据；
- P0/P1 为零，P2 有负责人、影响说明和后续版本计划；
- Slint 与第三方依赖许可、隐私说明、签名证书和时间戳链均通过审查；
- 72 小时后台稳定性、10,000 次保存压力、30 个长截图样本和安装生命周期测试通过；
- 在支持矩阵内至少完成一次干净 Windows 10 与 Windows 11 的签名安装包验收。

---

## 18. 建议的首批 Issue

按执行顺序创建：

1. `chore: initialize Cargo workspace, pinned MSVC toolchain and Windows CI`
2. `legal: review Slint distribution option and third-party licenses`
3. `spike: validate Slint overlay, pin windows, IME and HWND integration`
4. `spike: validate WGC and DXGI capture on mixed-DPI monitors`
5. `feat(domain): introduce desktop/monitor/image/logical coordinate types`
6. `feat(platform): enumerate displays and top-level windows`
7. `feat(app): native notification window, secure IPC, tray lifecycle and shutdown`
8. `feat(platform): register global hotkeys and user startup setting`
9. `feat(capture): implement threaded capture service abstraction`
10. `feat(ui-slint): implement fullscreen selection overlay`
11. `feat(capture): window highlighting and region crop`
12. `feat(output): clipboard and image encoders`
13. `build: bootstrap Inno Setup per-user installer and lifecycle smoke test`
14. `feat(annotation): vector model and undo/redo command stack`
15. `feat(annotation): basic shapes and export renderer`
16. `feat(pin): multi-window pin manager`
17. `feat(color): magnifier and color formats`
18. `feat(security): origin-bound Windows Credential Manager storage`
19. `feat(ai): OpenAI Responses and compatible Chat Completions adapters`
20. `feat(ai): OCR and translation result UI`
21. `feat(longshot): manual frame capture session`
22. `feat(longshot): overlap estimation and stitching`
23. `feat(longshot): automatic scrolling and fallback UX`
24. `test: add mixed-DPI, golden-image and background soak suites`
25. `release: sign Inno Setup exe and run install lifecycle gates`

---

## 19. 开工前必须确认的决策

已确定的决策：

- [x] UI 框架采用 Slint，特殊窗口能力使用 `windows` crate 接入 Win32。
- [x] 应用采用单实例、系统托盘后台常驻模式。
- [x] 首版仅发布 Windows x64 的 Inno Setup 单文件 `.exe` 安装包。
- [x] 默认执行当前用户级安装，不要求管理员权限。

在 M0 结束前仍需关闭以下决策，否则不进入大规模功能开发：

- [ ] Slint 渲染后端、版本及标注画布是否需要自定义渲染。
- [ ] 首选捕获 API 和回退顺序。
- [ ] 内部统一像素格式。
- [ ] 标注坐标保存方式。
- [ ] 最低 Windows 10 build 及 WGC/DXGI/GDI 能力回退矩阵。
- [ ] Slint 分发许可路径及第三方许可证合规结论。
- [ ] Windows 代码签名证书来源、CI 密钥保管与 RFC 3161 时间戳服务。
- [ ] Inno Setup AppId、Publisher、安装文件名和升级兼容策略。
- [ ] AI API 请求兼容范围及已验证服务列表。
- [ ] v1.0 是否只提供手动检查更新（推荐），以及截图历史是否继续留在 v1.x。
- [ ] HDR 场景是正式支持还是明确标记为实验性。

---

## 20. 推荐执行原则

1. **先打通截图主链路，再做丰富工具。** 普通截图的速度、坐标和稳定性是所有功能的基础。
2. **Slint 已定，先验证高风险能力再铺开页面。** 先通过透明覆盖层、多钉图、IME、HWND 集成和后台资源测试；不足部分在平台层或局部渲染器补足。
3. **长截图单独作为高风险子项目。** 建立真实样本集和量化指标，不以少量演示页面判断成功。
4. **AI 能力必须可替换。** 不让业务代码依赖某个模型名或单一服务商响应细节。
5. **隐私默认优先。** 截图只在用户主动执行 OCR/翻译时上传，Key 使用系统凭据存储。
6. **平台细节集中封装。** Win32、COM、WGC、DXGI 和 `unsafe` 代码不得扩散到 UI 与业务层。
7. **每个里程碑都交付可运行版本。** 避免直到最后才进行真实 Windows 环境集成。
