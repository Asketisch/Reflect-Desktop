//! MCP 协议版本常量与握手 helpers。
//!
//! 当前 reflect-mcp 固定使用 MCP `2025-06-18` spec。rmcp 1.7.0 内置
//! `ProtocolVersion::V_2025_06_18`,我们这里再以字符串常量形式暴露
//! 给 logging / 配置校验使用。

/// 我们声明并期望 server 使用的 MCP 协议版本。
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// 转换成 rmcp 的强类型 `ProtocolVersion`。
pub fn rmcp_protocol_version() -> rmcp::model::ProtocolVersion {
    rmcp::model::ProtocolVersion::V_2025_06_18
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_is_2025_06_18() {
        assert_eq!(PROTOCOL_VERSION, "2025-06-18");
    }

    #[test]
    fn rmcp_protocol_version_matches_constant() {
        let v = rmcp_protocol_version();
        assert_eq!(v, rmcp::model::ProtocolVersion::V_2025_06_18);
    }
}
