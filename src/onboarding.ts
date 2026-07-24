import { mount } from "svelte";
import "./styles.css";
import "./window.css";
import Onboarding from "./Onboarding.svelte";

export default mount(Onboarding, {
  target: document.getElementById("onboarding")!,
});
