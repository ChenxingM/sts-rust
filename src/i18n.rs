//! i18n module - internationalization support

#[derive(Clone, Copy, PartialEq)]
pub enum Language { En, Zh, Ja }

pub struct Translation {
    // Menu & General
    pub menu_file: &'static str, pub menu_edit: &'static str, pub menu_help: &'static str,
    pub action_new: &'static str, pub action_open: &'static str, pub action_save_as: &'static str, 
    pub action_export: &'static str, pub action_close_all: &'static str, pub action_settings: &'static str, pub action_about: &'static str,

    // Dialogs
    pub dialog_unsaved_title: &'static str, pub dialog_unsaved_body: &'static str,
    pub dialog_clear_title: &'static str, pub dialog_clear_body: &'static str, pub btn_clear_confirm: &'static str,

    // Buttons
    pub btn_create: &'static str, pub btn_cancel: &'static str, pub btn_save: &'static str, 
    pub btn_save_all: &'static str, pub btn_discard_all: &'static str, pub btn_dont_save: &'static str, 
    pub btn_ok: &'static str, pub btn_clear_all: &'static str,

    // 工具栏按钮
    pub btn_player_open: &'static str, pub btn_player_close: &'static str, pub btn_curve_tool: &'static str,

    // New Document
    pub label_name: &'static str, pub label_layers: &'static str, pub label_fps: &'static str, 
    pub label_f_per_page: &'static str, pub label_duration: &'static str, pub label_total: &'static str, pub label_pages: &'static str,

    // Info Bar
    pub info_layer: &'static str, pub info_frame: &'static str, pub info_page: &'static str,

    // Context Menu
    pub ctx_copy: &'static str, pub ctx_cut: &'static str, pub ctx_paste: &'static str, pub ctx_undo: &'static str, 
    pub ctx_repeat: &'static str, pub ctx_reverse: &'static str, pub ctx_smart_fill: &'static str, 
    pub ctx_sequence_fill: &'static str, pub ctx_copy_ae: &'static str, pub ctx_insert_col_l: &'static str, 
    pub ctx_insert_col_r: &'static str, pub ctx_del_col: &'static str,

    // Dialogs specific
    pub dialog_repeat_count: &'static str, pub dialog_repeat_until_end: &'static str,
    pub dialog_seq_start: &'static str, pub dialog_seq_end: &'static str, pub dialog_seq_hold: &'static str,

    // Settings
    pub settings_title: &'static str, pub settings_csv: &'static str, pub settings_general: &'static str, 
    pub settings_autosave: &'static str, pub settings_appearance: &'static str, pub settings_language: &'static str, pub settings_theme: &'static str,

    // Curve Editor
    pub curve_title: &'static str, pub curve_section_selection: &'static str, pub curve_target_layer: &'static str, 
    pub curve_frame_range: &'static str, pub curve_no_selection: &'static str, pub curve_no_selection_tip: &'static str, 
    pub curve_btn_linear: &'static str, pub curve_btn_ease_in: &'static str, pub curve_btn_ease_out: &'static str, 
    pub curve_btn_ease_in_out: &'static str, pub curve_label_start: &'static str, pub curve_label_duration: &'static str, 
    pub curve_label_drawings: &'static str, pub curve_info_ratio: &'static str, pub curve_btn_apply: &'static str,

    // === 新增：播放器 ===
    pub player_title: &'static str,
    pub player_play: &'static str, pub player_pause: &'static str, pub player_stop: &'static str,
    pub player_loop: &'static str, pub player_source: &'static str, pub player_ref_video: &'static str,
    pub player_bind_folder: &'static str, pub player_timeline: &'static str,
    pub player_bake: &'static str,
    

    // === 新增：主题设置 ===
    pub theme_customize: &'static str, pub theme_base_mode: &'static str, pub theme_dark_mode: &'static str,
    pub theme_save_as: &'static str, pub theme_save_btn: &'static str,

    // === 新增：悬浮提示与状态信息 ===
    pub hover_export: &'static str,
    pub hover_clear: &'static str,
    pub hover_bake: &'static str,
    pub msg_saved: &'static str,
    pub msg_cleared: &'static str,
}

pub const EN_US: Translation = Translation {
    menu_file: "File", menu_edit: "Edit", menu_help: "Help",
    action_new: "New Document", action_open: "Open...", action_save_as: "Save As...", action_export: "Export CSV...", action_close_all: "Close All", action_settings: "Settings", action_about: "About",
    dialog_unsaved_title: "Unsaved Changes", dialog_unsaved_body: "The following documents have unsaved changes:",
    dialog_clear_title: "Clear All Data?", dialog_clear_body: "This will erase ALL data in the current sheet.\nThis action can be undone.", btn_clear_confirm: "Yes, Clear All",
    btn_create: "Create", btn_cancel: "Cancel", btn_save: "Save", btn_save_all: "Save All", btn_discard_all: "Discard All", btn_dont_save: "Don't Save", btn_ok: "OK", btn_clear_all: "Clear All",
    btn_player_open: "Open Preview", btn_player_close: "Close Preview", btn_curve_tool: "Curve Tool",
    label_name: "Name:", label_layers: "Layers:", label_fps: "FPS:", label_f_per_page: "F/Page:", label_duration: "Duration:", label_total: "Total", label_pages: "Pages",
    info_layer: "Layer", info_frame: "Frame", info_page: "Page",
    ctx_copy: "Copy", ctx_cut: "Cut", ctx_paste: "Paste", ctx_undo: "Undo", ctx_repeat: "Repeat...", ctx_reverse: "Reverse", ctx_smart_fill: "Smart Fill", ctx_sequence_fill: "Sequence Fill...", ctx_copy_ae: "Copy AE Data",
    ctx_insert_col_l: "Insert Col Left", ctx_insert_col_r: "Insert Col Right", ctx_del_col: "Delete Column",
    dialog_repeat_count: "Count:", dialog_repeat_until_end: "Until End", dialog_seq_start: "Start:", dialog_seq_end: "End:", dialog_seq_hold: "Hold:",
    settings_title: "Preferences", settings_csv: "CSV Export", settings_general: "General", settings_autosave: "Auto-save on modify", settings_appearance: "Appearance", settings_language: "Language", settings_theme: "Theme",
    curve_title: "Curve Editor", curve_section_selection: "Active Selection", curve_target_layer: "Target Layer:", curve_frame_range: "Frame Range:", curve_no_selection: "⚠ No Selection", curve_no_selection_tip: "Please select a cell or range.", curve_btn_linear: "Linear", curve_btn_ease_in: "Ease In", curve_btn_ease_out: "Ease Out", curve_btn_ease_in_out: "Ease InOut", curve_label_start: "Start No.:", curve_label_duration: "Duration:", curve_label_drawings: "Drawings:", curve_info_ratio: "Avg: 1 drawing per {:.1} frames", curve_btn_apply: "Apply Curve",
    player_title: "Preview Player",player_play: "⏵ Play", player_pause: "⏸ Pause", player_stop: "⏹ Stop", player_loop: "Loop", player_source: "Source:", player_ref_video: "Ref Video", player_bind_folder: "📂 Bind Folder", player_timeline: "Timeline",player_bake: "Bake",
    theme_customize: "Customize Theme Colors", theme_base_mode: "Base Mode:", theme_dark_mode: "Dark UI Base", theme_save_as: "Save As:", theme_save_btn: "Save JSON",hover_export: "Export Timesheet to CSV format",
    hover_clear: "Clear all cells in this sheet",
    hover_bake: "Bake this layer to a sequence folder",
    msg_saved: "Document saved successfully.",
    msg_cleared: "Sheet cleared.",
};

pub const ZH_CN: Translation = Translation {
    menu_file: "文件", menu_edit: "编辑", menu_help: "帮助",
    action_new: "新建文档", action_open: "打开...", action_save_as: "另存为...", action_export: "导出 CSV...", action_close_all: "关闭所有", action_settings: "设置", action_about: "关于",
    dialog_unsaved_title: "未保存的更改", dialog_unsaved_body: "以下文档有未保存的更改:",
    dialog_clear_title: "清空所有数据？", dialog_clear_body: "这将清除当前表单的所有数据。\n此操作可以撤销。", btn_clear_confirm: "确认清空",
    btn_create: "创建", btn_cancel: "取消", btn_save: "保存", btn_save_all: "保存所有", btn_discard_all: "放弃更改", btn_dont_save: "不保存", btn_ok: "确定", btn_clear_all: "清空",
    btn_player_open: "开启预览", btn_player_close: "关闭预览", btn_curve_tool: "曲线工具",
    label_name: "名称:", label_layers: "层数:", label_fps: "帧率:", label_f_per_page: "一页帧数:", label_duration: "时长:", label_total: "总计", label_pages: "页数",
    info_layer: "层", info_frame: "帧", info_page: "页",
    ctx_copy: "复制", ctx_cut: "剪切", ctx_paste: "粘贴", ctx_undo: "撤销", ctx_repeat: "重复...", ctx_reverse: "倒序", ctx_smart_fill: "智能填充", ctx_sequence_fill: "序列填充...", ctx_copy_ae: "复制 AE 数据",
    ctx_insert_col_l: "左侧插入列", ctx_insert_col_r: "右侧插入列", ctx_del_col: "删除当前列",
    dialog_repeat_count: "次数:", dialog_repeat_until_end: "直到结束", dialog_seq_start: "开始值:", dialog_seq_end: "结束值:", dialog_seq_hold: "保持帧:",
    settings_title: "首选项", settings_csv: "CSV 导出设置", settings_general: "常规", settings_autosave: "修改时自动保存", settings_appearance: "外观", settings_language: "语言", settings_theme: "主题",
    curve_title: "曲线工具", curve_section_selection: "当前指向", curve_target_layer: "目标图层:", curve_frame_range: "帧范围:", curve_no_selection: "⚠ 无选区", curve_no_selection_tip: "请在表中选择单元格或范围。", curve_btn_linear: "线性", curve_btn_ease_in: "缓入", curve_btn_ease_out: "缓出", curve_btn_ease_in_out: "缓入缓出", curve_label_start: "起始号:", curve_label_duration: "持续帧:", curve_label_drawings: "张数:", curve_info_ratio: "平均: 每 {:.1} 帧 1 张", curve_btn_apply: "应用曲线",
    player_title: "预览器 (Preview Player)",player_play: "⏵ 播放", player_pause: "⏸ 暂停", player_stop: "⏹ 停止", player_loop: "循环", player_source: "源:", player_ref_video: "参考视频", player_bind_folder: "📂 绑定序列文件夹", player_timeline: "时间轴",player_bake: " 烘焙 ",
    theme_customize: "自定义主题颜色 (Customize Colors)", theme_base_mode: "基础模式:", theme_dark_mode: "深色 UI 底色 (Dark Mode)", theme_save_as: "另存为:", theme_save_btn: "保存主题 (JSON)",hover_export: "将摄影表导出为 CSV 格式",
    hover_clear: "清空当前表的所有数据",
    hover_bake: "将该层原画渲染为物理序列帧",
    msg_saved: "文档保存成功。",
    msg_cleared: "表单已清空。",
};

pub const JA_JP: Translation = Translation {
    menu_file: "ファイル", menu_edit: "編集", menu_help: "ヘルプ",
    action_new: "新規作成", action_open: "開く...", action_save_as: "名前を付けて保存...", action_export: "CSVエクスポート...", action_close_all: "すべて閉じる", action_settings: "設定", action_about: "バージョン情報",
    dialog_unsaved_title: "未保存の変更", dialog_unsaved_body: "以下のドキュメントは保存されていません:",
    dialog_clear_title: "全データをクリア？", dialog_clear_body: "現在のシートの全データが消去されます。\nこの操作は取り消せます。", btn_clear_confirm: "はい、クリア",
    btn_create: "作成", btn_cancel: "キャンセル", btn_save: "保存", btn_save_all: "すべて保存", btn_discard_all: "変更を破棄", btn_dont_save: "保存しない", btn_ok: "OK", btn_clear_all: "クリア",
    btn_player_open: "プレビュー開始", btn_player_close: "プレビュー終了", btn_curve_tool: "カーブツール",
    label_name: "名前:", label_layers: "レイヤー数:", label_fps: "FPS:", label_f_per_page: "1Pのコマ数:", label_duration: "長さ:", label_total: "合計", label_pages: "ページ数",
    info_layer: "レイヤー", info_frame: "フレーム", info_page: "ページ",
    ctx_copy: "コピー", ctx_cut: "切り取り", ctx_paste: "貼り付け", ctx_undo: "元に戻す", ctx_repeat: "繰り返し...", ctx_reverse: "反転", ctx_smart_fill: "スマートフィル", ctx_sequence_fill: "連番フィル...", ctx_copy_ae: "AEデータをコピー",
    ctx_insert_col_l: "左に列を挿入", ctx_insert_col_r: "右に列を挿入", ctx_del_col: "列を削除",
    dialog_repeat_count: "回数:", dialog_repeat_until_end: "最後まで", dialog_seq_start: "開始値:", dialog_seq_end: "終了値:", dialog_seq_hold: "コマ打ち:",
    settings_title: "設定", settings_csv: "CSVエクスポート", settings_general: "一般", settings_autosave: "変更時に自動保存", settings_appearance: "外観", settings_language: "言語", settings_theme: "テーマ",
    curve_title: "カーブエディタ", curve_section_selection: "現在の選択", curve_target_layer: "対象レイヤー:", curve_frame_range: "フレーム範囲:", curve_no_selection: "⚠ 選択なし", curve_no_selection_tip: "セルまたは範囲を選択してください。", curve_btn_linear: "リニア", curve_btn_ease_in: "イーズイン", curve_btn_ease_out: "イーズアウト", curve_btn_ease_in_out: "イーズイン/アウト", curve_label_start: "開始番号:", curve_label_duration: "長さ:", curve_label_drawings: "枚数:", curve_info_ratio: "平均: {:.1} フレームに1枚", curve_btn_apply: "適用",
    player_title: "プレビューア (Preview Player)",player_play: "⏵ 再生", player_pause: "⏸ 一時停止", player_stop: "⏹ 停止", player_loop: "ループ", player_source: "ソース:", player_ref_video: "参考動画", player_bind_folder: "📂 フォルダをリンク", player_timeline: "タイムライン",player_bake: "ベイク",
    theme_customize: "テーマカラーのカスタマイズ", theme_base_mode: "ベースモード:", theme_dark_mode: "ダークモード UI", theme_save_as: "名前を付けて保存:", theme_save_btn: "保存する (JSON)",hover_export: "タイムシートをCSV形式でエクスポート",
    hover_clear: "このシートのすべてのセルをクリア",
    hover_bake: "このレイヤーを連番画像としてベイク",
    msg_saved: "ドキュメントを保存しました。",
    msg_cleared: "シートをクリアしました。",
};

