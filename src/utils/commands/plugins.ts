/**
 * 插件 IPC 封装 —— composer 斜杠菜单的插件命令数据源。
 *
 * 只读快照:后端从挂载好的插件运行时取 slash 命令注册表(state/plugins.rs);
 * 未挂载 / 无 enabled 插件时返回空数组。选中命令后由 Composer 以
 * `/name args` 形态提交,展开仍由后端 submit 边界统一做(expand_submission)。
 */
import { invoke } from '../bridge';

export interface ReflectPluginCommandInfo {
  /** 命令全名(`plugin:ns:name` 形式,如 `demo:hello`),前端拼 `/<name>`。 */
  name: string;
  /** 命令说明(md frontmatter `description`,可缺省)。 */
  description: string | null;
}

/** 列出当前已挂载插件的全部 slash 命令。 */
export async function reflect_list_plugin_commands(): Promise<ReflectPluginCommandInfo[]> {
  return invoke<ReflectPluginCommandInfo[]>('reflect_list_plugin_commands');
}
