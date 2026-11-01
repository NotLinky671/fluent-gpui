#!/usr/bin/env node
// Snapshots @fluentui/tokens into vendor/tokens.json.
//
// The package is not vendored: fetch and extract it first, then point this
// script at the extracted directory.
//
//   npm pack @fluentui/tokens --pack-destination "$TMP"
//   tar -xzf "$TMP"/fluentui-tokens-*.tgz -C "$TMP"
//   node scripts/snapshot-tokens.mjs "$TMP/package" vendor/tokens.json
//
// Output shape:
//   { source, version, themes: { webLight: { token: value } }, brands: { webLight: 80, ... } }
// where `brands` maps brand-derived tokens to the ramp stop they were built
// from, so the Rust side can regenerate them for a custom ramp.

import { createRequire } from 'node:module';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';

const pkgDir = process.argv[2];
const outPath = process.argv[3] ?? 'vendor/tokens.json';
if (!pkgDir) {
  console.error('usage: snapshot-tokens.mjs <extracted-package-dir> [out.json]');
  process.exit(2);
}

const require = createRequire(import.meta.url);
const pkg = JSON.parse(readFileSync(join(pkgDir, 'package.json'), 'utf8'));
const m = require(join(pkgDir, 'lib-commonjs', 'index.cjs'));

const THEMES = {
  webLight: m.webLightTheme,
  webDark: m.webDarkTheme,
  teamsLight: m.teamsLightTheme,
  teamsDark: m.teamsDarkTheme,
  webHighContrast: m.teamsHighContrastTheme,
};

// Brand ramps, from packages/tokens/src/global/brandColors.ts.
const RAMPS = {
  WEB: ['#061724','#082338','#0a2e4a','#0c3b5e','#0e4775','#0f548c','#115ea3','#0f6cbd','#2886de','#479ef5','#62abf5','#77b7f7','#96c6fa','#b4d6fa','#cfe4fa','#ebf3fc'],
  TEAMS: ['#2b2b40','#2f2f4a','#333357','#383966','#3d3e78','#444791','#4f52b2','#5b5fc7','#7579eb','#7f85f5','#9299f7','#aab1fa','#b6bcfa','#c5cbfa','#dce0fa','#e8ebfa'],
};

const BRAND_OF = {
  webLight: 'WEB', webDark: 'WEB',
  teamsLight: 'TEAMS', teamsDark: 'TEAMS',
  webHighContrast: null,
};

function rampStop(ramp, value) {
  const index = RAMPS[ramp]?.findIndex((hex) => hex.toLowerCase() === String(value).toLowerCase());
  return index === undefined || index < 0 ? null : (index + 1) * 10;
}

const themes = {};
const brands = {};
for (const [name, theme] of Object.entries(THEMES)) {
  if (!theme) throw new Error(`missing theme ${name}`);
  themes[name] = { ...theme };
  brands[name] = {};
  const ramp = BRAND_OF[name];
  if (!ramp) continue;
  for (const [token, value] of Object.entries(theme)) {
    if (!token.includes('Brand')) continue;
    const stop = rampStop(ramp, value);
    if (stop !== null) brands[name][token] = stop;
  }
}

const tokenNames = [...new Set(Object.values(themes).flatMap((t) => Object.keys(t)))].sort();
for (const name of Object.keys(themes)) {
  const missing = tokenNames.filter((t) => !(t in themes[name]));
  if (missing.length) throw new Error(`theme ${name} is missing ${missing.length} tokens: ${missing.slice(0, 5)}`);
}

const out = {
  source: `@fluentui/tokens@${pkg.version}`,
  themes,
  brands,
  tokenNames,
};

mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, JSON.stringify(out, null, 2) + '\n');
console.log(`wrote ${outPath}: ${tokenNames.length} tokens x ${Object.keys(themes).length} themes`);
for (const [name, map] of Object.entries(brands)) {
  console.log(`  ${name}: ${Object.keys(map).length} brand-derived tokens`);
}