use serde::{Deserialize, Serialize};

/// 关键帧：表示某一帧的时间值
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    /// 帧编号
    pub frame: i32,
    /// 时间值（秒）
    pub time: f64,
}

impl Keyframe {
    pub fn new(frame: i32, time: f64) -> Self {
        Self { frame, time }
    }
}

/// 时间重映射：包含多个关键帧的集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRemap {
    pub keyframes: Vec<Keyframe>,
}

impl TimeRemap {
    pub fn new() -> Self {
        Self {
            keyframes: Vec::new(),
        }
    }

    pub fn add_keyframe(&mut self, frame: i32, time: f64) {
        self.keyframes.push(Keyframe { frame, time });
        // 保持按帧号排序
        self.keyframes.sort_by_key(|k| k.frame);
    }

    /// 根据帧号插值计算时间
    /// 如果帧号在关键帧之间，进行线性插值
    pub fn interpolate(&self, frame: i32) -> Option<f64> {
        if self.keyframes.is_empty() {
            return None;
        }

        // 查找前后关键帧
        let mut prev_kf: Option<&Keyframe> = None;
        let mut next_kf: Option<&Keyframe> = None;

        for kf in &self.keyframes {
            if kf.frame == frame {
                return Some(kf.time);
            } else if kf.frame < frame {
                prev_kf = Some(kf);
            } else if kf.frame > frame && next_kf.is_none() {
                next_kf = Some(kf);
                break;
            }
        }

        // 线性插值
        match (prev_kf, next_kf) {
            (Some(prev), Some(next)) => {
                let t = (frame - prev.frame) as f64 / (next.frame - prev.frame) as f64;
                Some(lerp(prev.time, next.time, t))
            }
            (Some(prev), None) => Some(prev.time), // 超出范围，使用最后一帧
            (None, Some(next)) => Some(next.time), // 超出范围，使用第一帧
            (None, None) => None,
        }
    }

    /// 获取时间范围
    pub fn time_range(&self) -> Option<(f64, f64)> {
        if self.keyframes.is_empty() {
            return None;
        }
        let min = self.keyframes.iter().map(|k| k.time).fold(f64::INFINITY, f64::min);
        let max = self.keyframes.iter().map(|k| k.time).fold(f64::NEG_INFINITY, f64::max);
        Some((min, max))
    }

    /// 获取帧范围
    pub fn frame_range(&self) -> Option<(i32, i32)> {
        if self.keyframes.is_empty() {
            return None;
        }
        let min = self.keyframes.iter().map(|k| k.frame).min()?;
        let max = self.keyframes.iter().map(|k| k.frame).max()?;
        Some((min, max))
    }
}

impl Default for TimeRemap {
    fn default() -> Self {
        Self::new()
    }
}

/// 线性插值函数
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate() {
        let mut tr = TimeRemap::new();
        tr.add_keyframe(0, 0.0);
        tr.add_keyframe(10, 1.0);
        tr.add_keyframe(20, 2.0);

        assert_eq!(tr.interpolate(0), Some(0.0));
        assert_eq!(tr.interpolate(5), Some(0.5));
        assert_eq!(tr.interpolate(10), Some(1.0));
        assert_eq!(tr.interpolate(15), Some(1.5));
        assert_eq!(tr.interpolate(20), Some(2.0));
    }

    #[test]
    fn test_time_range() {
        let mut tr = TimeRemap::new();
        tr.add_keyframe(0, 5.0);
        tr.add_keyframe(10, 1.0);
        tr.add_keyframe(20, 10.0);

        assert_eq!(tr.time_range(), Some((1.0, 10.0)));
    }

    #[test]
    fn test_frame_range() {
        let mut tr = TimeRemap::new();
        tr.add_keyframe(5, 0.0);
        tr.add_keyframe(10, 1.0);
        tr.add_keyframe(20, 2.0);

        assert_eq!(tr.frame_range(), Some((5, 20)));
    }
}
