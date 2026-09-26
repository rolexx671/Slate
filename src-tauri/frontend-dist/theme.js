// Runs before stylesheets so the first rendered frame uses the saved appearance.
(() => {
 'use strict';
 const settingsKey = 'slate-ru-settings:v1';
 const media = window.matchMedia('(prefers-color-scheme: dark)');
 const normalize = value => ['dark', 'light', 'system'].includes(value) ? value : 'dark';
 const read = () => {
  try { return normalize(JSON.parse(localStorage.getItem(settingsKey) || '{}').theme); }
  catch { return 'dark'; }
 };
 let preference = read();
 function render() {
  document.documentElement.dataset.theme = preference === 'system' ? (media.matches ? 'dark' : 'light') : preference;
  document.documentElement.style.colorScheme = document.documentElement.dataset.theme;
 }
 function apply(value) {
  preference = normalize(value);
  render();
  const invoke = window.__TAURI__?.core?.invoke;
  if (invoke) void invoke('set_native_theme', { theme: preference }).catch(error => console.warn('Native appearance:', error));
 }
 media.addEventListener('change', () => { if (preference === 'system') render(); });
 window.addEventListener('storage', event => {
  if (event.key !== settingsKey) return;
  apply(read());
  window.dispatchEvent(new CustomEvent('slate-theme-synced', { detail: preference }));
 });
 window.SlateTheme = Object.freeze({ apply, normalize, getPreference: () => preference });
 render();
})();
