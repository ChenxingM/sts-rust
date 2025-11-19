use serde::{Deserialize, Serialize};
use super::keyframe::TimeRemap;

/// 图层：包含时间重映射数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub time_remap: TimeRemap,
}

impl Layer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            time_remap: TimeRemap::new(),
        }
    }

    pub fn with_time_remap(name: impl Into<String>, time_remap: TimeRemap) -> Self {
        Self {
            name: name.into(),
            time_remap,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_creation() {
        let layer = Layer::new("Test Layer");
        assert_eq!(layer.name, "Test Layer");
        assert!(layer.time_remap.keyframes.is_empty());
    }
}
