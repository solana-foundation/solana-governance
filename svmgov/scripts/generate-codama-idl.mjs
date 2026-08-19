import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { createFromRoot } from "codama";

const packageDir = dirname(dirname(fileURLToPath(import.meta.url)));
const anchorIdlPath = resolve(
  packageDir,
  "program/target/idl/svmgov_program.json",
);
const codamaIdlPath = resolve(packageDir, "idl/codama.json");
const anchorIdl = await readFile(anchorIdlPath, "utf8");
const codama = createFromRoot(rootNodeFromAnchor(JSON.parse(anchorIdl)));

await mkdir(dirname(codamaIdlPath), { recursive: true });
await writeFile(codamaIdlPath, `${codama.getJson()}\n`);
console.log(`Wrote ${codamaIdlPath}`);
