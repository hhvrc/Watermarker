// Tauri serves a static bundle with no Node server, so prerender to plain HTML
// (adapter-static) and disable SSR.
export const prerender = true;
export const ssr = false;
