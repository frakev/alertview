// Resolves the theme before the first paint, otherwise a light-theme user gets
// a flash of the dark palette hardcoded on <html>.
//
// A separate file rather than an inline <script> so the Content-Security-Policy
// can be `script-src 'self'` with no `unsafe-inline` — which is what actually
// blocks an injected `onmouseover=` handler. Loaded blocking in <head>: it must
// run before anything is painted.
(function () {
  try {
    var pref = localStorage.getItem('av-theme') || 'auto';
    var dark = pref === 'auto'
      ? matchMedia('(prefers-color-scheme: dark)').matches
      : pref === 'dark';
    document.documentElement.setAttribute('data-theme', dark ? 'dark' : 'light');
  } catch (e) {}
})();
