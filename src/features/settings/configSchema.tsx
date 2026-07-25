/**
 * Compatibility re-exports for the legacy `./configSchema` import.
 *
 * The structured configuration schema has been split across:
 *   - `./config/schema.ts`  — pure types + TOML scalar field helpers
 *   - `./config/toml.ts`    — pure TOML section / flag helpers
 *   - `./components/StructuredField.tsx` — React field renderer
 *
 * Existing callers (`import { ... } from '@/features/settings/configSchema'`)
 * keep working unchanged.
 */

export {
  COMPLEX_SECTIONS,
  SIMPLE_FIELDS_BY_SECTION,
  STRUCTURED_SECTIONS,
  applyField,
  readField,
  type FieldKind,
  type FieldSpec,
} from './config/schema';
export { StructuredField, type StructuredFieldProps } from './components/StructuredField';