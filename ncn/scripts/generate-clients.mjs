import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { renderVisitor } from "@codama/renderers-js";
import { createFromJson } from "codama";

const packageDir = dirname(dirname(fileURLToPath(import.meta.url)));
const codamaIdl = await readFile(resolve(packageDir, "idl/codama.json"), "utf8");
const codama = createFromJson(codamaIdl);

await codama.accept(
  renderVisitor(resolve(packageDir, "generated/clients/ts"), {
    deleteFolderBeforeRendering: true,
    formatCode: true,
    generatedFolder: "",
    kitImportStrategy: "preferRoot",
    syncPackageJson: false,
  }),
);
