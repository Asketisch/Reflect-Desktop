/**
 * Media Studio + Computer Use IPC 包装器（Phase 3 条目 13）。
 *
 * 5 个命令:list_media / image_process / screenshot / computer_use /
 * media_capabilities。
 */
import { invoke } from '@/utils/bridge';

export type ReflectImageFormat = 'png' | 'jpeg' | 'gif' | 'webp' | 'bmp';

export interface ReflectMediaAsset {
  path: string;
  filename: string;
  sizeBytes: number;
  mimeType?: string | null;
  width?: number | null;
  height?: number | null;
  modifiedAtMs: number;
}

export interface ReflectImageProcessSpec {
  inputPath: string;
  outputPath: string;
  format?: ReflectImageFormat | null;
  width?: number | null;
  height?: number | null;
  quality?: number | null;
}

export interface ReflectImageProcessResult {
  outputPath: string;
  width: number;
  height: number;
  format: ReflectImageFormat;
  bytes: number;
}

export type ReflectComputerUseAction =
  | { kind: 'screenshot'; params: null }
  | { kind: 'mouseMove'; params: { x: number; y: number } }
  | { kind: 'mouseClick'; params: { x: number; y: number; button?: string } }
  | { kind: 'keyType'; params: { text: string } }
  | { kind: 'keyCombo'; params: { keys: string } }
  | { kind: 'scroll'; params: { dx: number; dy: number } };

export interface ReflectMediaCapabilities {
  imageBackend: string;
  computerBackend: string;
  note: string;
}

export function reflect_list_media(dir: string): Promise<ReflectMediaAsset[]> {
  return invoke('reflect_list_media', { dir });
}

export function reflect_image_process(spec: ReflectImageProcessSpec): Promise<ReflectImageProcessResult> {
  return invoke('reflect_image_process', { spec });
}

export function reflect_screenshot(): Promise<string> {
  return invoke('reflect_screenshot');
}

export function reflect_computer_use(action: ReflectComputerUseAction): Promise<void> {
  return invoke('reflect_computer_use', { action });
}

export function reflect_media_capabilities(): Promise<ReflectMediaCapabilities> {
  return invoke('reflect_media_capabilities');
}
