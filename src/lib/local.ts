// Whether an address is on the local network, which Android 17 lets an
// app reach only with the user's permission. Tailscale's 100.64.0.0/10 and
// public addresses aren't. A host name could be either, so only `.local`
// names count; a name that turns out local fails to connect, and saying so
// is the link bar's job.

export function isLocal(address: string): boolean {
  const a = address.replace(/^\[|\]$/g, "").toLowerCase();
  if (a.endsWith(".local") || a === "localhost") return true;
  const v4 = a.match(/^(\d+)\.(\d+)\.\d+\.\d+$/);
  if (v4) {
    const [x, y] = [Number(v4[1]), Number(v4[2])];
    return x === 10 || (x === 172 && y >= 16 && y < 32) || (x === 192 && y === 168) || (x === 169 && y === 254);
  }
  // Link-local and unique local IPv6.
  return a.includes(":") && /^(fe[89ab]|f[cd])/.test(a);
}
