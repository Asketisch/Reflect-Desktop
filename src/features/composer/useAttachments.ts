/**
 * useAttachments —— Composer 的附件状态管理 (B5)。
 *
 * 模型: 一组 "pending" UserInputItem (本地路径 / inline image / skill mention)。
 * 在用户点 Send 时,把所有 pending items + 文本组成 `userInput(...)` submission。
 *
 * 不直接调 `reflect_*` —— 暴露 `items` + `text` 给 caller 自行组装。
 *
 * CodexMonitor 同名: `src/features/composer/hooks/useAttachments.ts`
 */
import { useCallback, useState } from 'react';
import type { UserInputItem } from '@/types/protocol';

export type PendingAttachment =
  | { kind: 'local_image'; path: string }
  | { kind: 'image'; data: string; mime_type: string }
  | { kind: 'skill'; name: string };

export interface UseAttachmentsResult {
  attachments: PendingAttachment[];
  addLocalImage: (path: string) => void;
  addInlineImage: (data: string, mimeType: string) => void;
  addSkillMention: (name: string) => void;
  remove: (idx: number) => void;
  clear: () => void;
  toUserInputItems: () => UserInputItem[];
}

export function useAttachments(): UseAttachmentsResult {
  const [attachments, setAttachments] = useState<PendingAttachment[]>([]);

  const addLocalImage = useCallback((path: string) => {
    setAttachments((prev) => [...prev, { kind: 'local_image', path }]);
  }, []);

  const addInlineImage = useCallback((data: string, mimeType: string) => {
    setAttachments((prev) => [...prev, { kind: 'image', data, mime_type: mimeType }]);
  }, []);

  const addSkillMention = useCallback((name: string) => {
    setAttachments((prev) => [...prev, { kind: 'skill', name }]);
  }, []);

  const remove = useCallback((idx: number) => {
    setAttachments((prev) => prev.filter((_, i) => i !== idx));
  }, []);

  const clear = useCallback(() => setAttachments([]), []);

  const toUserInputItems = useCallback((): UserInputItem[] => {
    return attachments.map((a): UserInputItem => {
      switch (a.kind) {
        case 'local_image':
          return { type: 'local_image', path: a.path };
        case 'image':
          return { type: 'image', data: a.data, mime_type: a.mime_type };
        case 'skill':
          return { type: 'skill', name: a.name };
      }
    });
  }, [attachments]);

  return {
    attachments,
    addLocalImage,
    addInlineImage,
    addSkillMention,
    remove,
    clear,
    toUserInputItems,
  };
}
