import { defineConfig } from "oxlint";

import solanaConfig from "@solana-config/oxc/oxlint";

export default defineConfig({
  extends: [solanaConfig],
  rules: {
    "no-unused-vars": "error",
    "sort-keys": "warn",
  },
});
