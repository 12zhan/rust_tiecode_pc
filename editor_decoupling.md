# 代码编辑器组件解耦与基础架构说明

## 目标

本阶段目标不是性能极限优化或功能扩展，而是：

> **打好结构基础，降低未来复杂度，引导正确的依赖方向**

核心原则：

- 解耦 ≠ 分文件 ≠ 抽象过度
- 解耦 = 改变依赖方向
- 允许全量计算，但必须是**显式、可控的全量计算**

---

## 一、当前结构现状

当前 `CodeEditor` 同时承担三类职责：

1. **编辑器核心逻辑（Model / Core）**
   - 文本内容
   - 光标 / 选区
   - 插入 / 删除
   - UTF-8 索引

2. **平台与输入适配（Adapter）**
   - IME / EntityInputHandler
   - UTF-16 ↔ UTF-8 转换
   - 坐标与字符索引换算

3. **UI 与渲染（View）**
   - gpui Render
   - canvas 绘制
   - 滚动、字体、bounds

在 demo / 初期阶段这是**完全合理的**，但它形成了一个未来的复杂度中心点。

---

## 二、解耦的核心判断标准

判断是否需要解耦，不看“卡不卡”，而看这三个问题：

1. render 路径是否只负责绘制？
2. 派生数据是否有明确生命周期？
3. 一次编辑操作，理论上最大影响面是否可描述？

解耦的目标是让这些问题**有结构性答案**。

---

## 三、最小侵入式解耦原则

### 原则 1：核心逻辑不依赖 UI 框架

- 核心逻辑不应依赖：
  - gpui 类型
  - Pixels / Bounds
  - WindowContext

### 原则 2：UI 只是“壳”

- UI 层负责：
  - 事件
  - 坐标
  - 通知 repaint
- 不负责语义决策

### 原则 3：允许全量，但必须显式

- 可以 O(N)
- 但不能“隐式 O(N)”

---

## 四、最小解耦结构草图

### 1. EditorCore（纯核心）

```rust
struct EditorCore {
    content: String,
    selection: Range<usize>,
    selection_anchor: usize,
    marked_range: Option<Range<usize>>,
}
```

特点：
- 只处理 UTF-8
- 不知道 UI、滚动、字体
- 可单元测试

---

### 2. CodeEditor（UI 壳）

```rust
struct CodeEditor {
    core: EditorCore,

    // UI / 平台相关
    focus_handle: FocusHandle,
    scroll_offset: Point<Pixels>,
    font_size: Pixels,
    last_bounds: Option<Bounds<Pixels>>,
}
```

职责：
- 转发输入
- 调用 core
- 触发 notify / repaint

---

### 3. 输入适配层（逻辑边界）

```text
EditorCore  <-- UTF-8 -->  InputAdapter  <-- UTF-16 / gpui -->
```

- UTF 转换
- 坐标 ↔ 索引
- IME 生命周期

这是**天然的隔离层**。

---

## 五、关于性能的正确定位

当前实现：

- 在中小规模文本下性能是**完全成立的**
- 高亮与输入不卡是正常现象

本阶段不是为了“提速”，而是为了：

> **让未来的性能优化有落点，而不是推倒重来**

---

## 六、完成解耦后的判断标准

当你能回答“是”时，说明解耦到位：

- 是否可以在没有 gpui 的 crate 中复用核心编辑逻辑？
- render 是否只消费准备好的数据？
- 编辑影响面是否可理论描述？

---

## 七、建议的阶段性停点

当前阶段，推荐做到：

- ✅ 拆出 EditorCore
- ✅ 明确 Input / UI / Core 边界
- ❌ 不引入 trait
- ❌ 不做多模块重构
- ❌ 不做复杂缓存

这是**最小、正确、可回退**的一步。

---

## 结语

这一步不是为了“看起来高级”，而是为了：

> **让系统在复杂之前，就已经站在正确的结构上。**
