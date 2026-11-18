export interface Appearance {
  mode: 'icons' | 'single';
  accent: string;
  glow: boolean;
  glowColor: string;
  overrides: Record<string, string>;
}

export type ItemKind = 'application' | 'file' | 'folder';

export const defaultAppearance: Appearance = {
  mode: 'icons',
  accent: '#497FBE',
  glow: true,
  glowColor: '#497FBE',
  overrides: {},
};

const fallbacks = { application: '#497FBE', file: '#647E9B', folder: '#3C8891' };
const accents = new Map<string, Promise<string>>();

export const validHex = (value: string) => /^#[0-9a-f]{6}$/i.test(value);

export function cardAccent(settings: Appearance, kind: ItemKind, id?: string, sampled?: string) {
  const hex = (id && Object.hasOwn(settings.overrides, id) && settings.overrides[id])
    || (settings.mode === 'single' ? settings.accent : sampled || fallbacks[kind]);
  const safeHex = validHex(hex) ? hex : fallbacks[kind];
  const channels = [1, 3, 5].map((offset) => Number.parseInt(safeHex.slice(offset, offset + 2), 16));
  const highest = Math.max(...channels);
  const lowest = Math.min(...channels);

  // Keep all endpoints lighter than the base but dark enough for secondary text.
  const muted = channels.map((channel) => Math.round(40 + 42 * (channel - lowest) / Math.max(65, highest - lowest)));
  return `rgb(${muted.join(', ')})`;
}

export function iconAccent(source: string): Promise<string> {
  const cached = accents.get(source);
  if (cached) return cached;
  if (accents.size >= 1024) {
    const oldest = accents.keys().next();
    if (!oldest.done) accents.delete(oldest.value);
  }
  const result = sampleIcon(source).catch(() => fallbacks.application);
  accents.set(source, result);
  return result;
}

async function sampleIcon(source: string) {
  const image = new Image();
  image.src = source;
  await image.decode();
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 48;
  const context = canvas.getContext('2d');
  if (!context) return fallbacks.application;
  context.drawImage(image, 0, 0, 48, 48);

  const { data } = context.getImageData(0, 0, 48, 48);
  const buckets = Array.from({ length: 12 }, () => ({ weight: 0, sums: [0, 0, 0] }));
  for (let index = 0; index < data.length; index += 4) {
    const [r, g, b, alpha] = data.subarray(index, index + 4);
    const high = Math.max(r, g, b);
    const low = Math.min(r, g, b);
    const chroma = high - low;
    if (alpha < 150 || high < 42 || low > 218 || chroma < 25 || chroma / high < 0.19) continue;

    const hue = high === r ? (g - b) / chroma : high === g ? 2 + (b - r) / chroma : 4 + (r - g) / chroma;
    const bucket = buckets[Math.round((hue + 6) * 2) % buckets.length];
    const weight = chroma * alpha;
    bucket.weight += weight;
    [r, g, b].forEach((channel, channelIndex) => { bucket.sums[channelIndex] += channel * weight; });
  }
  const dominant = buckets.reduce((best, current) => current.weight > best.weight ? current : best);
  if (!dominant.weight) return fallbacks.application;
  return '#' + dominant.sums.map((sum) => Math.round(sum / dominant.weight).toString(16).padStart(2, '0')).join('');
}
