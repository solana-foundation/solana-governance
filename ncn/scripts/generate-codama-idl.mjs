import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import {
  addPdasVisitor,
  constantPdaSeedNodeFromString,
  createFromRoot,
  publicKeyTypeNode,
  variablePdaSeedNode,
} from "codama";

const packageDir = dirname(dirname(fileURLToPath(import.meta.url)));
const anchorIdlPath = resolve(packageDir, "target/idl/ncn_snapshot.json");
const codamaIdlPath = resolve(packageDir, "idl/codama.json");
const anchorIdl = await readFile(anchorIdlPath, "utf8");
const codama = createFromRoot(rootNodeFromAnchor(JSON.parse(anchorIdl)));

codama.update(
  addPdasVisitor({
    ncnSnapshot: [
      {
        name: "metaMerkleProof",
        seeds: [
          constantPdaSeedNodeFromString("utf8", "MetaMerkleProof"),
          variablePdaSeedNode("consensusResult", publicKeyTypeNode()),
          variablePdaSeedNode("voteAccount", publicKeyTypeNode()),
        ],
      },
    ],
  }),
);

await mkdir(dirname(codamaIdlPath), { recursive: true });
await writeFile(codamaIdlPath, `${codama.getJson()}\n`);
console.log(`Wrote ${codamaIdlPath}`);
