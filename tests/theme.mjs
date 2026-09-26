import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const code = fs.readFileSync(new URL('../src-tauri/frontend-dist/theme.js', import.meta.url), 'utf8');
function start(saved, systemDark = false) {
 const root = { dataset: {}, style: {} }, events = {}, nativeCalls = [];
 const media = { matches: systemDark, addEventListener: (_, listener) => { media.listener = listener; } };
 const storage = { getItem: () => saved };
 const window = { matchMedia: () => media,
  addEventListener: (name, listener) => { events[name] = listener; },
  dispatchEvent: event => { window.lastEvent = event; },
  __TAURI__: { core: { invoke: async (name, args) => { nativeCalls.push([name, args.theme]); } } }
 };
 const ctx = { window, localStorage: storage, document: { documentElement: root }, console,
  CustomEvent: class { constructor(type, init) { this.type=type; this.detail=init.detail; } } };
 vm.runInNewContext(code, ctx);
 return { root, media, window, storage, events, nativeCalls, api: window.SlateTheme };
}
for (const saved of [null, '{}', '{broken', '{"theme":"unknown"}']) {
 assert.equal(start(saved).root.dataset.theme, 'dark');
}
const explicit = start('{"theme":"light"}', true);
assert.equal(explicit.root.dataset.theme, 'light');
assert.equal(explicit.root.style.colorScheme, 'light');
explicit.media.matches = false; explicit.media.listener();
assert.equal(explicit.root.dataset.theme, 'light');
explicit.api.apply('dark');
assert.equal(explicit.root.dataset.theme, 'dark');
assert.deepEqual(explicit.nativeCalls.at(-1), ['set_native_theme', 'dark']);
explicit.api.apply('system');
assert.equal(explicit.root.dataset.theme, 'light');
assert.deepEqual(explicit.nativeCalls.at(-1), ['set_native_theme', 'system']);
explicit.media.matches = true; explicit.media.listener();
assert.equal(explicit.root.dataset.theme, 'dark');
explicit.storage.getItem = () => '{"theme":"light"}';
explicit.events.storage({key:'slate-ru-settings:v1'});
assert.equal(explicit.root.dataset.theme, 'light');
assert.equal(explicit.window.lastEvent.detail, 'light');
assert.equal(explicit.api.normalize('unsupported'), 'dark');
console.log('Theme tests passed: bootstrap, saved choice, system changes, window sync and native appearance.');
