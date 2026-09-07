# Snipe 系统架构文档

本文档描述 Snipe 截图工具的分层设计、模块职责与并发通信流。

当前实现缺口与修复顺序见 [`repair-plan.md`](repair-plan.md)；审计结论见 [`implementation-status.md`](implementation-status.md)。

## 1. 架构拓扑

```text
┌────────────────────────────────────────────────────────┐
│ UI / Slint 展示层                                      │
│ overlay.slint, pin.slint, editor.slint, settings.slint │
└───────────────────────────┬────────────────────────────┘
                            │ Slint event loop & callbacks
┌───────────────────────────▼────────────────────────────┐
│ Controller & Application Services                      │
│ AppController, PinManager, LongshotSession             │
└───────┬──────────────┬────────────┬─────────────┬──────┘
        │              │            │             │
┌───────▼──────┐ ┌─────▼────┐ ┌─────▼────┐ ┌──────▼─────┐
│ Capture      │ │ Imaging  │ │ Annotate │ │ AI Client  │
│ Abstraction  │ │ Pipeline │ │ Model    │ │ Provider   │
└───────┬──────┘ └──────────┘ └──────────┘ └────────────┘
        │
┌───────▼────────────────────────────────────────────────┐
│ Platform Layer (Windows)                               │
│ Win32 Message Pump, Global Hotkeys, GDI/DXGI Capture,  │
│ System Tray, Windows Credential Manager, Named Pipes   │
└────────────────────────────────────────────────────────┘
```

## 2. 核心 Crate 分工

1. **`domain`**：基础数据类型、强类型几何坐标 (`DesktopPxRect`, `ImagePxRect`, `LogicalRect`)、色彩模型 (`ColorRgba`, `ColorHsl`, `ColorHsv`) 及原始 `Frame` 抽象。
2. **`config`**：用户非敏感配置 (快捷键、自动启动、保存路径、AI 模型参数)。
3. **`secure-storage-windows`**：调用 Windows Credential Manager 进行 API Key 的加密读写，杜绝落盘泄漏。
4. **`capture-core`**：异步 `CaptureService` 契约抽象与显示器/窗口信息模型。
5. **`capture-windows`**：Windows 平台捕获服务实现，集成 GDI/BitBlt 与 DWM 扩展边界检测。
6. **`imaging`**：图像格式编码 (PNG/JPEG/WebP)、原子保存、局部模糊、马赛克及放大镜网格生成。
7. **`annotation`**：矢量标注图元、撤销/重做命令栈、以及 1:1 像素离屏渲染合成器。
8. **`longshot`**：多帧滚动位移估计算法与长图拼接状态机。
9. **`ai-client`**：标准 VisionProvider，适配 OpenAI 与 OpenAI-compatible 多模态端点。
10. **`ui-slint`**：Slint 界面组件与 Rust 数据桥接。
11. **`app`**：程序入口、单实例互斥锁、系统托盘、Win32 消息泵及全局快捷键分发。
