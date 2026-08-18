/**
 * Skill 注册表封装 —— 列出已安装 skill 及其触发词。
 */
import { invoke } from '../bridge';

export interface ReflectSkillInfo {
  name: string;
  description: string;
  path: string;
  tools: string[];
  triggers: string[];
}

/** 列出全部已安装 skill。 */
export async function reflect_list_skills(): Promise<ReflectSkillInfo[]> {
  return invoke<ReflectSkillInfo[]>('reflect_list_skills');
}
