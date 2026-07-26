/**
 * MentionPicker —— `@` 弹层,列出 skills (B5)。
 *
 * 客户端 fetch reflect_list_skills,渲染为可过滤的列表;
 * 点击 → onPick(name),外部 caller 把 { type: 'skill', name } 加入 attachments。
 */
import { useEffect, useMemo, useState } from 'react';
import { reflect_list_skills, type ReflectSkillInfo } from '@/utils/commands';
import s from './MentionPicker.module.css';

export interface MentionPickerProps {
  visible: boolean;
  query: string;
  onPick: (name: string) => void;
  onClose: () => void;
}

export function MentionPicker({ visible, query, onPick, onClose }: MentionPickerProps) {
  const [skills, setSkills] = useState<ReflectSkillInfo[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!visible) return;
    let alive = true;
    setLoading(true);
    reflect_list_skills()
      .then((list) => {
        if (alive) setSkills(list);
      })
      .catch(() => {
        if (alive) setSkills([]);
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [visible]);

  const filtered = useMemo(() => {
    const q = query.toLowerCase();
    if (!q) return skills;
    return skills.filter(
      (sk) =>
        sk.name.toLowerCase().includes(q) ||
        sk.description.toLowerCase().includes(q),
    );
  }, [skills, query]);

  if (!visible) return null;

  return (
    <div className={s.popup} role="listbox" aria-label="Skills" data-testid="mention-picker">
      {loading ? (
        <div className={s.empty}>Loading skills…</div>
      ) : filtered.length === 0 ? (
        <div className={s.empty}>No matching skills.</div>
      ) : (
        filtered.map((sk) => (
          <button
            key={sk.name}
            type="button"
            role="option"
            className={s.item}
            onClick={() => {
              onPick(sk.name);
              onClose();
            }}
            data-testid={`mention-skill-${sk.name}`}
          >
            <div className={s.name}>/{sk.name}</div>
            <div className={s.desc}>{sk.description}</div>
          </button>
        ))
      )}
    </div>
  );
}
