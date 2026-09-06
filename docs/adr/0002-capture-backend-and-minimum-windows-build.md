# ADR 0002: 屏幕捕获后端策略与最低 Windows 版本基线

## 状态
已采纳 (Accepted)

## 上下文
Windows 10 和 11 提供了多种捕获 API，包括 GDI `BitBlt`、DXGI Desktop Duplication、以及 Windows Graphics Capture (WGC)。

## 决策
1. **MVP 优先基线**：以 GDI `BitBlt(SRCCOPY | CAPTUREBLT)` 作为最广泛兼容的捕获实现，确保在各种硬件、核显、独显及虚拟机远程桌面环境中稳定工作。
2. **架构预留**：通过 `CaptureService` trait 将捕获后端与业务解耦，后续可无缝增加 WGC 现代硬件加速后端。
3. **最低版本**：Windows 10 1809 及以上（全量支持 Per-Monitor DPI V2 与 DWM 扩展边界检测）。
