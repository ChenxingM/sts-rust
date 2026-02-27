use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use std::env;

/// 将视频导入并转换为序列帧缓存
/// [新增] 增加了 target_fps 参数，动态匹配当前摄影表的帧率
pub fn extract_frames(video_path: &str, output_parent_dir: &Path, target_fps: u32) -> Result<PathBuf, String> {
    let video_path_obj = Path::new(video_path);
    let stem = video_path_obj.file_stem().unwrap().to_str().unwrap();
    
    // 创建缓存文件夹，并加上帧率后缀，防止不同帧率混用缓存
    let seq_dir_name = format!("{}_{}fps_seq", stem, target_fps);
    let output_dir = output_parent_dir.join(seq_dir_name);

    if !output_dir.exists() {
        fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    }

    let output_pattern = output_dir.join("frame_%04d.png");
    
    let ffmpeg_exe_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let mut ffmpeg_path = ffmpeg_exe_name.to_string(); 

    if let Ok(mut exe_dir) = env::current_exe() {
        exe_dir.pop(); 
        exe_dir.push(ffmpeg_exe_name); 

        if exe_dir.exists() {
            ffmpeg_path = exe_dir.to_string_lossy().to_string();
        }
    }

    // 动态拼接 fps 参数
    let fps_arg = format!("fps={}", target_fps);

    let status = Command::new(&ffmpeg_path)
        .arg("-i")
        .arg(video_path)
        .arg("-vf")
        .arg(&fps_arg) // 使用文档的真实帧率
        .arg(output_pattern)
        .status();

    match status {
        Ok(s) if s.success() => Ok(output_dir),
        Ok(_) => Err("FFmpeg 异常退出。请确认视频文件正常。".to_string()),
        Err(e) => Err(format!("无法调用 ffmpeg，请确保软件目录下存在 ffmpeg.exe！\n详细错误: {}", e)),
    }
}