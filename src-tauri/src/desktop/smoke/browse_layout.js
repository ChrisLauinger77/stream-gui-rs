// Exercise the production stylesheet with synthetic cards/rows, without Twitch.
// Hidden descriptions below a long list must not enlarge the outer document.
(async () => {
  const root = document.getElementById("root");
  const originalScale = document.documentElement.dataset.textScale;
  const fixture = document.createElement("div");
  root.hidden = true;
  document.body.append(fixture);
  const results = [];
  try {
    for (const scale of ["100", "125", "150"]) {
      document.documentElement.dataset.textScale = scale;
      for (const kind of ["stream", "channel"]) {
        const items = Array.from({ length: 40 }, (_, index) => {
          const description = `<span class="sr-only" id="description-${index}">Live · Synthetic category</span>`;
          const button = `<button class="item-link ${kind === "channel" ? "channel-row" : ""}" aria-describedby="description-${index}">${kind === "stream" ? '<span class="media"></span>' : ""}<span><h3>Synthetic channel ${index}</h3><p class="stream-title">Synthetic title</p></span></button>`;
          return `<article class="${kind === "stream" ? "stream-card" : "channel-entry"}">${button}${description}<button class="watch-button">Watch</button></article>`;
        }).join("");
        fixture.innerHTML = `<div class="application"><header class="app-bar">Header</header><div class="workspace"><nav class="side-nav">Navigation</nav><main class="browse-content"><h1>Following</h1><div class="${kind === "stream" ? "stream-grid" : "channel-list"}">${items}</div></main></div><footer class="app-footer"><span>Footer</span><button>About</button></footer></div>`;
        const pane = fixture.querySelector("main");
        const height = innerHeight;
        const documentHeight = document.documentElement.scrollHeight;
        scrollTo(0, documentHeight);
        const outerScroll = document.scrollingElement.scrollTop;
        pane.scrollTop = pane.scrollHeight;
        const innerScroll = pane.scrollTop;
        pane.scrollTop = 0;
        const lastButton = pane.querySelector("article:last-child .watch-button");
        lastButton.focus();
        await new Promise(resolve => requestAnimationFrame(resolve));
        results.push({
          width: innerWidth, scale, kind, height, documentHeight, outerScroll, innerScroll,
          focused: document.activeElement === lastButton,
          focusedScroll: pane.scrollTop,
          focusedOuterScroll: document.scrollingElement.scrollTop,
          headerTop: fixture.querySelector("header").getBoundingClientRect().top,
          footerBottom: fixture.querySelector("footer").getBoundingClientRect().bottom,
        });
      }
    }
    return results;
  } finally {
    fixture.remove();
    root.hidden = false;
    if (originalScale === undefined) delete document.documentElement.dataset.textScale;
    else document.documentElement.dataset.textScale = originalScale;
    scrollTo(0, 0);
  }
})
