use std::collections::HashMap;
use std::path::PathBuf;

use crate::commands::HookInfo;

pub(crate) struct HookStore {
    path: Option<PathBuf>,
}

impl Default for HookStore {
    fn default() -> Self {
        Self {
            path: dirs::home_dir().map(|home| home.join(".reflect/hook_state.json")),
        }
    }
}

impl HookStore {
    pub(crate) fn list(&self) -> anyhow::Result<Vec<HookInfo>> {
        let state = self.load_state();
        Ok(vec![
            HookInfo {
                name: "read_before_edit".to_string(),
                kind: "policy".to_string(),
                enabled: state.get("read_before_edit").copied().unwrap_or(true),
                config_summary: "auto-reads file before Edit/Write".to_string(),
            },
            HookInfo {
                name: "plan_mode_gate".to_string(),
                kind: "policy".to_string(),
                enabled: state.get("plan_mode_gate").copied().unwrap_or(true),
                config_summary: "blocks mutating tools in Plan mode".to_string(),
            },
        ])
    }

    pub(crate) fn toggle(&self, name: String, enabled: bool) -> anyhow::Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no home dir"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut state = self.load_state();
        state.insert(name, enabled);
        std::fs::write(path, serde_json::to_string_pretty(&state)?)?;
        Ok(())
    }

    fn load_state(&self) -> HashMap<String, bool> {
        self.path
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_toggle_is_reflected_in_listing() {
        let path =
            std::env::temp_dir().join(format!("reflect-hook-state-{}.json", uuid::Uuid::new_v4()));
        let store = HookStore {
            path: Some(path.clone()),
        };
        store.toggle("read_before_edit".into(), false).unwrap();

        let hooks = store.list().unwrap();
        assert!(
            !hooks
                .iter()
                .find(|hook| hook.name == "read_before_edit")
                .unwrap()
                .enabled
        );
        assert!(
            hooks
                .iter()
                .find(|hook| hook.name == "plan_mode_gate")
                .unwrap()
                .enabled
        );

        let _ = std::fs::remove_file(path);
    }
}
