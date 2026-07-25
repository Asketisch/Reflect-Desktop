/**
 * Workspace I/O — list / set / query the active workspace.
 */
import { invoke } from '../bridge';

export interface ReflectWorkspaceInfo {
  path: string;
  label: string;
  last_used: number;
  session_count: number;
}

/** List known workspaces. */
export async function reflect_list_workspaces(): Promise<ReflectWorkspaceInfo[]> {
  return invoke<ReflectWorkspaceInfo[]>('reflect_list_workspaces');
}

/** Set the active workspace by absolute path. */
export async function reflect_set_workspace(path: string): Promise<void> {
  return invoke<void>('reflect_set_workspace', { path });
}

/** Return the absolute path of the active workspace. */
export async function reflect_current_workspace(): Promise<string> {
  return invoke<string>('reflect_current_workspace');
}
