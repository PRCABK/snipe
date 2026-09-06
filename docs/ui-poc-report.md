# Slint UI & Win32 集成验证报告 (PoC)

## 1. 验证目标

评估 Slint 1.9 与 Win32 原生基础设施在 Windows 10/11 平台上的协同表现：
- 后台无窗口驻留与托盘图标生命周期
- 全局快捷键 (`WM_HOTKEY`) 响应延迟
- 多钉图窗口独立置顶与透明度控制
- 图像像素转换开销与离屏渲染合成

## 2. 结论与关键决策

1. **事件循环与原生消息协调**：
   - 采用单独后台任务轮询或 Win32 专用通知窗口分发消息，通过 `slint::invoke_from_event_loop` 将任务提交至 UI 线程，完全避免跨线程并发访问 UI 对象的问题。
2. **像素格式与内存传输**：
   - 使用 `slint::SharedPixelBuffer<slint::Rgba8Pixel>` 作为零成本或低成本内存桥接方式，将 `domain::Frame` 转换为 `slint::Image`，兼顾速度与内存稳定性。
3. **单实例保护**：
   - 命名互斥量 `Local\Snipe_SingleInstance_Mutex` 配合 Windows 命名管道，支持秒级快速唤起已存在的实例，二次启动绝不弹出重复托盘图标。
