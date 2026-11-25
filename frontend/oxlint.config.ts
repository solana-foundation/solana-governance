import { defineConfig } from "oxlint";

import solanaConfig from "@solana-config/oxc/oxlint";

export default defineConfig({
  extends: [solanaConfig],
  plugins: ["typescript", "unicorn", "oxc", "react", "nextjs"],
  rules: {
    "no-unused-vars": "error",
    "react/exhaustive-deps": "warn",
    "react/rules-of-hooks": "error",
    // Key order carries no meaning in this codebase, and its warnings bury
    // the ones that do.
    "sort-keys": "off",
  },
});
