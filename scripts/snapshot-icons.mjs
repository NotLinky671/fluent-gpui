#!/usr/bin/env node
// Snapshots a curated subset of Fluent System Icons (MIT) into vendor/icons.json.
//
// Icons are embedded as raw SVG bytes and rendered with `svg().data(..)`, so no
// AssetSource registration is needed and the application keeps its own assets.
//
//   node scripts/snapshot-icons.mjs vendor/icons.json
//
// Add an icon by adding an entry to ICONS below; keep the list to what the
// components actually use (ADR-0005).

import { writeFileSync, mkdirSync } from 'node:fs';
import { dirname } from 'node:path';

const outPath = process.argv[2] ?? 'vendor/icons.json';

const REPO = 'microsoft/fluentui-system-icons';
const REF = 'main';
const SIZE = 20;

// snake name -> repository display name
const ICONS = {
  add: 'Add',
  arrow_left: 'Arrow Left',
  arrow_right: 'Arrow Right',
  checkmark: 'Checkmark',
  checkmark_circle: 'Checkmark Circle',
  chevron_down: 'Chevron Down',
  chevron_left: 'Chevron Left',
  chevron_right: 'Chevron Right',
  chevron_up: 'Chevron Up',
  dismiss: 'Dismiss',
  dismiss_circle: 'Dismiss Circle',
  error_circle: 'Error Circle',
  home: 'Home',
  info: 'Info',
  more_horizontal: 'More Horizontal',
  person: 'Person',
  search: 'Search',
  settings: 'Settings',
  subtract: 'Subtract',
  warning: 'Warning',
};

const STYLES = ['regular', 'filled'];

async function fetchSvg(displayName, style) {
  const file = `ic_fluent_${displayName.toLowerCase().replaceAll(' ', '_')}_${SIZE}_${style}.svg`;
  const url = `https://raw.githubusercontent.com/${REPO}/${REF}/assets/${encodeURIComponent(displayName)}/SVG/${file}`;
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${response.status} for ${url}`);
  const text = await response.text();
  if (!text.startsWith('<svg')) throw new Error(`not an svg: ${url}`);
  return text;
}

const icons = {};
for (const [name, displayName] of Object.entries(ICONS)) {
  icons[name] = {};
  for (const style of STYLES) {
    icons[name][style] = await fetchSvg(displayName, style);
  }
  process.stdout.write('.');
}

const out = {
  source: `${REPO}@${REF}`,
  size: SIZE,
  icons,
};
mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, JSON.stringify(out, null, 2) + '\n');
console.log(`\nwrote ${outPath}: ${Object.keys(icons).length} icons x ${STYLES.length} styles`);