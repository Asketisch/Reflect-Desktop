/**
 * Ask-user wrappers — respond to questions raised by the agent via the
 * `reflect_event` stream.
 */
import { invoke } from '../bridge';

/** Answer a multi-choice question. Shape of `answers` is defined per question. */
export async function reflect_ask_user_question_response(id: string, answers: unknown): Promise<string> {
  return invoke<string>('reflect_ask_user_question_response', { id, answers });
}

/** Free-text answer to a clarification request. */
export async function reflect_ask_user_input_response(id: string, text: string): Promise<string> {
  return invoke<string>('reflect_ask_user_input_response', { id, text });
}
