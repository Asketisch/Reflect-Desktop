/**
 * LSP 开关 / 状态 / 预热封装。
 *
 * LSP 按工作区由用户手动开启（默认关闭）：开启后注册 `lsp` tool 并启动
 * `[lsp_servers]` 配置的语言服务；未开启时不注入工具、不做预热。
 * 开关偏好的持久化（localStorage，按 workspace 路径）在 GUI 层。
 */
import { invoke } from '../bridge';

/** LSP 当前状态。 */
export interface LspStatus {
  enabled: boolean;
  servers: string[];
}

/** 开启 / 关闭 LSP（对当前生效的工作区）。 */
export async function reflect_lsp_set_enabled(enabled: boolean): Promise<LspStatus> {
  return invoke<LspStatus>('reflect_lsp_set_enabled', { enabled });
}

/** 查询 LSP 状态。 */
export async function reflect_lsp_status(): Promise<LspStatus> {
  return invoke<LspStatus>('reflect_lsp_status');
}

/**
 * 预热一个代码文件（didOpen，server 侧建索引）。LSP 未开启或无匹配
 * server 时 warmed = false，不报错 —— 预热是尽力而为的后台优化。
 */
export async function reflect_lsp_warmup(
  path: string,
): Promise<{ warmed: boolean; server: string | null }> {
  return invoke<{ warmed: boolean; server: string | null }>('reflect_lsp_warmup', { path });
}
