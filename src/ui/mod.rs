//! UI module - contains UI rendering components

// 声明子模块 (告诉 Rust 这些文件的存在)
pub mod cell;
pub mod about;
pub mod player;
pub mod curve_editor; // <--- 这一行是关键！没有它就会报你现在的错

// 导出内部结构 (方便外部使用)
pub use cell::{render_cell, CellColors};
pub use about::AboutDialog;
pub use player::SequencePlayer;
pub use curve_editor::CurveEditor; // <--- 这一行让你可以用 crate::ui::CurveEditor