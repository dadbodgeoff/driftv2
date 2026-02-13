/**
 * Table output format — human-readable terminal output.
 */

interface TableRow {
  [key: string]: unknown;
}

/**
 * Format data as a human-readable table.
 */
export function formatTable(data: unknown): string {
  if (data === null || data === undefined) {
    return '';
  }

  // Handle arrays of objects as table rows
  if (Array.isArray(data)) {
    if (data.length === 0) return 'No results.\n';
    if (typeof data[0] === 'object' && data[0] !== null) {
      return renderObjectTable(data as TableRow[]);
    }
    return data.map(String).join('\n') + '\n';
  }

  // Handle single objects as key-value pairs
  if (typeof data === 'object') {
    return renderKeyValue(data as Record<string, unknown>);
  }

  return String(data) + '\n';
}

function renderObjectTable(rows: TableRow[]): string {
  const keys = Object.keys(rows[0]);
  const widths = keys.map((k) =>
    Math.max(k.length, ...rows.map((r) => formatCellValue(r[k]).length)),
  );

  const lines: string[] = [];

  // Header
  const header = keys.map((k, i) => k.padEnd(widths[i])).join('  ');
  lines.push(header);
  lines.push(widths.map((w) => '─'.repeat(w)).join('──'));

  // Rows
  for (const row of rows) {
    const line = keys
      .map((k, i) => formatCellValue(row[k]).padEnd(widths[i]))
      .join('  ');
    lines.push(line);
  }

  return lines.join('\n') + '\n';
}

/**
 * Format a cell value for table display.
 * Arrays are shown as counts, objects are summarized, long strings truncated.
 */
function formatCellValue(v: unknown): string {
  if (v === null || v === undefined) return '';
  if (Array.isArray(v)) {
    if (v.length === 0) return '0';
    // For arrays of primitives, show compact comma list
    if (typeof v[0] !== 'object') return v.join(', ');
    // For arrays of objects (e.g. matches), show count
    return String(v.length);
  }
  if (typeof v === 'object') {
    // Flatten simple objects into "key=value, key=value" format
    // instead of dumping raw JSON.
    const entries = Object.entries(v as Record<string, unknown>);
    if (entries.length === 0) return '{}';
    const flat = entries.map(([k, val]) => {
      if (val === null || val === undefined) return `${k}=—`;
      if (typeof val === 'number') return `${k}=${Number.isInteger(val) ? val : val.toFixed(3)}`;
      if (typeof val === 'boolean') return `${k}=${val}`;
      if (typeof val === 'string') return `${k}=${val.length > 20 ? val.slice(0, 17) + '...' : val}`;
      if (Array.isArray(val)) return `${k}=[${val.length}]`;
      // Nested object — just show key count
      return `${k}={${Object.keys(val as object).length}}`;
    }).join(', ');
    if (flat.length > 80) return flat.slice(0, 77) + '...';
    return flat;
  }
  const s = String(v);
  // Truncate very long values for table readability
  if (s.length > 80) return s.slice(0, 77) + '...';
  return s;
}

function renderKeyValue(obj: Record<string, unknown>): string {
  const entries = Object.entries(obj);
  if (entries.length === 0) return 'No data.\n';

  const maxKeyLen = Math.max(...entries.map(([k]) => k.length));
  return (
    entries
      .map(([k, v]) => `${k.padEnd(maxKeyLen)}  ${formatValue(v)}`)
      .join('\n') + '\n'
  );
}

function formatValue(v: unknown): string {
  if (v === null || v === undefined) return '—';
  if (typeof v === 'object') {
    // Use the same smart formatting as table cells
    return formatCellValue(v);
  }
  return String(v);
}
