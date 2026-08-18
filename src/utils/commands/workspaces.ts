/**
 * Workspace I/O —— 列出 / 设置 / 查询当前 workspace。
 */
import { invoke } from '../bridge';

export interface ReflectWorkspaceInfo {
  path: string;
  label: string;
  last_used: number;
  session_count: number;
}

/** 列出已知 workspace。 */
export async function reflect_list_workspaces(): Promise<ReflectWorkspaceInfo[]> {
  return invoke<ReflectWorkspaceInfo[]>('reflect_list_workspaces');
}

/** 按绝对路径设置当前 workspace。 */
export async function reflect_set_workspace(path: string): Promise<void> {
  return invoke<void>('reflect_set_workspace', { path });
}

/** 返回当前 workspace 的绝对路径。 */
export async function reflect_current_workspace(): Promise<string> {
  return invoke<string>('reflect_current_workspace');
}
