/**
 * 询问用户封装 —— 响应 agent 通过 `reflect_event` 流提出的问题。
 */
import { invoke } from '../bridge';

/** 回答一道多选题。`answers` 的形态由每道题自行定义。 */
export async function reflect_ask_user_question_response(id: string, answers: unknown): Promise<string> {
  return invoke<string>('reflect_ask_user_question_response', { id, answers });
}

/** 以自由文本回答一个澄清请求。 */
export async function reflect_ask_user_input_response(id: string, text: string): Promise<string> {
  return invoke<string>('reflect_ask_user_input_response', { id, text });
}
