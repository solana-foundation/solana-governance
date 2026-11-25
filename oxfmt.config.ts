import { defineConfig } from "oxfmt";
import solanaConfig from "@solana-config/oxc/oxfmt";

export default defineConfig({
  extends: [solanaConfig],
});