/**
 * Skill registry wrappers — list installed skills and their triggers.
 */
import { invoke } from '../bridge';

export interface ReflectSkillInfo {
  name: string;
  description: string;
  path: string;
  tools: string[];
  triggers: string[];
}

/** List all installed skills. */
export async function reflect_list_skills(): Promise<ReflectSkillInfo[]> {
  return invoke<ReflectSkillInfo[]>('reflect_list_skills');
}
