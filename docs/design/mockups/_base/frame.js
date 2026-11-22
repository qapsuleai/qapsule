// RD2/G — anchor frame plumbing (not product logic): reads the page parameters, links
// the look's generated variables and fonts, points the chart slots at the core-rendered
// images and marks the page ready for the screenshot.
//
//   ?look=<name>&theme=dark|light      which generated vars file to load
//   &vars=<url prefix>                 where `<look>-<theme>.css` and `fonts-<look>.css`
//                                      live (default `vars/`, next to the page)
//   &charts=<url prefix>&dpr=<n>       chart images `<prefix><slot>.png`, drawn at n×
//
// scripts/design-gallery passes all of them; it writes nothing into this folder.
(function () {
  const q = new URLSearchParams(location.search);
  const look = q.get('look') || 'r4';
  const theme = q.get('theme') === 'light' ? 'light' : 'dark';
  const vars = q.get('vars') || 'vars/';
  const charts = q.get('charts');
  const dpr = q.get('dpr') || '1';
  const root = document.documentElement;
  root.dataset.look = look;
  root.dataset.theme = theme;

  const link = (href) =>
    new Promise((done) => {
      const l = document.createElement('link');
      l.rel = 'stylesheet';
      l.href = href;
      l.onload = l.onerror = done;
      // Before the anchor's own stylesheets: they read the variables, a direction's
      // stylesheet (linked last in its copy of the page) may override them.
      document.head.prepend(l);
    });

  const images = () =>
    Promise.all(
      [...document.querySelectorAll('img[data-chart]')].map(
        (img) =>
          new Promise((done) => {
            if (!charts) return done();
            img.onload = img.onerror = done;
            img.srcset = `${charts}${img.dataset.chart}.png ${dpr}x`;
          })
      )
    );

  Promise.all([link(`${vars}fonts-${look}.css`), link(`${vars}${look}-${theme}.css`)])
    .then(() => document.fonts.ready)
    .then(images)
    .then(() => {
      root.dataset.ready = '1';
    });
})();
