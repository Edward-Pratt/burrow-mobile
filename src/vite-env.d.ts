/// <reference types="vite/client" />

// svelte-check types each component itself; this covers plain .ts
// importers under TypeScript 6's resolution.
declare module "*.svelte" {
  import type { Component } from "svelte";
  const component: Component;
  export default component;
}
