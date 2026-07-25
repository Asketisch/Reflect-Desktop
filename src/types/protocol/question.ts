/**
 * Reflect protocol — Question / QuestionOption / Answer / AskUserAnswer.
 *
 * Mirrors `vendor/reflect-protocol/src/question.rs` (structured
 * multi-question prompts). All wire-format field names are snake_case.
 *
 * See `./index.ts` for the top-level vendor-sync warning.
 */

export interface Question {
  header: string;
  question: string;
  options: QuestionOption[];
  multi_select: boolean;
}

export interface QuestionOption {
  label: string;
  description: string;
  preview?: string;
}

export interface Answer {
  selected?: number[];
  custom?: string;
}

export interface AskUserAnswer {
  answers: Answer[];
}
