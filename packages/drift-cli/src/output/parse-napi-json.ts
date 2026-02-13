/**
 * Parse a JSON string returned by NAPI bindings that serialize to String.
 * Returns the parsed object, or wraps the raw string in { raw: string }
 * so formatOutput always receives a structured value.
 */
export function parseNapiJson(raw: string): unknown {
  try {
    return JSON.parse(raw);
  } catch {
    return { raw };
  }
}
