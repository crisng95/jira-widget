import { mount } from "svelte";
import "./styles.css";
import "./settings.css";
import Settings from "./Settings.svelte";

export default mount(Settings, {
  target: document.getElementById("settings")!,
});
