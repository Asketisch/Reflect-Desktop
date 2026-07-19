//! 语音服务 stub —— STT/TTS MCP 集成占位。

/// 语音服务状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceServiceStatus {
    Disabled,
    StubEnabled,
}

/// 语音服务 stub。
#[derive(Debug, Clone, Default)]
pub struct VoiceService {
    pub enabled: bool,
    pub provider: Option<String>,
}

impl VoiceService {
    pub fn status(&self) -> VoiceServiceStatus {
        if self.enabled {
            VoiceServiceStatus::StubEnabled
        } else {
            VoiceServiceStatus::Disabled
        }
    }

    pub fn status_line(&self) -> String {
        match self.status() {
            VoiceServiceStatus::Disabled => {
                "voice-service: stub — 启用 [voice].enabled=true(v2.x Whisper/TTS)".into()
            }
            VoiceServiceStatus::StubEnabled => format!(
                "voice-service: stub — provider={:?}",
                self.provider.as_deref().unwrap_or("default")
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_status() {
        let v = VoiceService {
            enabled: true,
            provider: Some("whisper".into()),
        };
        assert_eq!(v.status(), VoiceServiceStatus::StubEnabled);
    }
}
