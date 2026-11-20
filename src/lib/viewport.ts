// The phone's keyboard covers the page rather than shrinking it, since
// Android draws apps edge to edge. The visual viewport does shrink, so the
// app is sized to it: a shell's last line and its key bar stay above the
// keyboard.

export function fitToKeyboard() {
  const viewport = window.visualViewport;
  if (!viewport) return;
  const fit = () => {
    document.documentElement.style.setProperty("--viewport-height", `${viewport.height}px`);
    window.scrollTo(0, 0);
  };
  viewport.addEventListener("resize", fit);
  fit();
}
