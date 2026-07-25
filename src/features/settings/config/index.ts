/**
 * Barrel re-exports for the pure configuration layer.
 *
 * Pulled out of the legacy `configSchema.tsx` + `ConfigForm.tsx` so
 * the schema metadata, the TOML string helpers, and the React
 * components can be tested and consumed independently.
 */

export * from './schema';
export * from './toml';