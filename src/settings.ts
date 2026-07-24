import { mount } from "svelte";
import "./styles.css";
import "./window.css";
import "./settings.css";
import Settings from "./Settings.svelte";

export default mount(Settings, {
  target: document.getElementById("settings")!,
});
