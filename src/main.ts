import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { fitToKeyboard } from "./lib/viewport";

fitToKeyboard();

const app = mount(App, { target: document.getElementById("app")! });

export default app;
