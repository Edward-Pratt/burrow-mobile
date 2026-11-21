// Typing into a shell from a phone keyboard. xterm's own input doesn't
// survive Android keyboards: they compose words and rewrite them, and
// xterm then sends letters twice or out of order. So a textarea of our
// own takes the keyboard, and what changed in its text is sent: letters
// added as themselves, letters taken away as backspaces. A word the
// keyboard corrects arrives as backspaces and the new word, as typed.

/// Keys with no text, as a terminal sends them.
const KEYS: Record<string, string> = {
  Enter: "\r",
  Tab: "\t",
  Escape: "\x1b",
  ArrowUp: "\x1b[A",
  ArrowDown: "\x1b[B",
  ArrowRight: "\x1b[C",
  ArrowLeft: "\x1b[D",
  Home: "\x1b[H",
  End: "\x1b[F",
  Delete: "\x1b[3~",
};

/// What turns `before` into `after`: a backspace for each letter lost
/// from the end, then the letters added.
export function difference(before: string, after: string): string {
  const a = [...before];
  const b = [...after];
  let same = 0;
  while (same < a.length && same < b.length && a[same] === b[same]) same++;
  return "\x7f".repeat(a.length - same) + b.slice(same).join("");
}

/// Sends what is typed into `input` with `send`. Returns the function
/// that stops it.
export function follow(input: HTMLTextAreaElement, send: (data: string) => void): () => void {
  let text = "";
  const clear = () => {
    input.value = "";
    text = "";
  };
  const onInput = () => {
    const now = input.value;
    const data = difference(text, now).replace(/\n/g, "\r");
    text = now;
    // A line break ends the line, and a long line is let go of, so the
    // keyboard's idea of the text never grows without end.
    if (now.includes("\n") || now.length > 100) clear();
    if (data) send(data);
  };
  const onKey = (e: KeyboardEvent) => {
    // A backspace in an empty box changes nothing, so it has no input.
    let key = e.key === "Backspace" && input.value === "" ? "\x7f" : KEYS[e.key];
    // Ctrl with a letter, from a keyboard that has one.
    if (e.ctrlKey && e.key.length === 1) key = String.fromCharCode(e.key.toUpperCase().charCodeAt(0) & 0x1f);
    if (key === undefined || e.isComposing) return;
    e.preventDefault();
    if (e.key === "Enter") clear();
    send(key);
  };
  input.addEventListener("input", onInput);
  input.addEventListener("keydown", onKey);
  return () => {
    input.removeEventListener("input", onInput);
    input.removeEventListener("keydown", onKey);
  };
}
