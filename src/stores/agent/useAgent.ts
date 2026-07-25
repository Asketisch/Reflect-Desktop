import { useAgentStore } from './store';

export function useAgent() {
  const turns = useAgentStore((state) => state.turns);
  const session = useAgentStore((state) => state.session);
  const submit = useAgentStore((state) => state.submit);
  return { turns, session, submit };
}
