/**
 * tasks-board feature barrel (Phase 1 multi-agent UI).
 */
export { TasksBoardView } from './TasksBoardView';
export { useTasksBoardController } from './useTasksBoardController';
export type {
  TasksBoardController,
  BoardView,
} from './useTasksBoardController';
export { DEFAULT_LIST_ID, STATUS_LABELS, STATUS_ORDER } from './useTasksBoardController';
