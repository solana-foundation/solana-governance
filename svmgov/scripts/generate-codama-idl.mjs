import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { createFromRoot } from "codama";
import { format } from "oxfmt";

const packageDir = dirname(dirname(fileURLToPath(import.meta.url)));
const anchorIdlPath = resolve(packageDir, "program/target/idl/svmgov_program.json");
const codamaIdlPath = resolve(packageDir, "idl/codama.json");
const anchorIdl = await readFile(anchorIdlPath, "utf8");
const codama = createFromRoot(rootNodeFromAnchor(JSON.parse(anchorIdl)));

await mkdir(dirname(codamaIdlPath), { recursive: true });
const { code, errors } = await format(codamaIdlPath, `${codama.getJson()}\n`);
if (errors.length > 0) {
  throw new Error(errors.map(({ message }) => message).join("\n"));
}
await writeFile(codamaIdlPath, code);
console.log(`Wrote ${codamaIdlPath}`);
