//! Feature Flag 运行时查询 helper。

use crate::schema::ReflectConfig;

/// 从完整配置查询 feature flag;未配置段 → false。
pub fn is_feature_enabled(cfg: &ReflectConfig, flag: &str) -> bool {
    cfg.feature_flags
        .as_ref()
        .is_some_and(|ff| ff.is_enabled(flag))
}

/// 合并 compile-time feature 与 runtime flag(两者都 true 才启用)。
pub fn is_feature_enabled_with_compile_time(
    cfg: &ReflectConfig,
    flag: &str,
    compile_time: bool,
) -> bool {
    compile_time && is_feature_enabled(cfg, flag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::FeatureFlagsSection;
    use std::collections::HashMap;

    #[test]
    fn missing_section_is_false() {
        assert!(!is_feature_enabled(&ReflectConfig::default(), "voice"));
    }

    #[test]
    fn explicit_true() {
        let cfg = ReflectConfig {
            feature_flags: Some(FeatureFlagsSection {
                flags: HashMap::from([("voice".into(), true)]),
            }),
            ..Default::default()
        };
        assert!(is_feature_enabled(&cfg, "voice"));
    }
}
