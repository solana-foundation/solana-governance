import {
  proposalRefFromFileName,
  type ProposalRef,
  type ProposalRepoConfig,
} from "./proposalUrl";

/** The subset of GitHub's "list pull request files" response that we rely on. */
export interface PullRequestFile {
  /** Repo-relative path, NOT percent-encoded. */
  filename: string;
  status: string;
  /** `.../contents/{encoded path}?ref={head_sha}` — the only place the head SHA appears. */
  contents_url: string;
}

export interface PickedProposalFile {
  path: string;
  headSha: string;
  ref: ProposalRef | undefined;
}

export type PickProposalFileResult =
  | { status: "ok"; file: PickedProposalFile }
  | { status: "none" }
  /** Several documents have equal claim; the description does not say which is meant. */
  | { status: "ambiguous"; paths: string[] };

/**
 * Picks the one proposal document out of a pull request's changed files.
 *
 * A real proposal PR touches more than the proposal: PR #3 on solana-governance-proposals
 * changes `README.md` and the root-level `XXXX-sgp-template.md` alongside
 * `proposals/sgp-0001-solana-constitution.md`. Only the last is the document we want.
 *
 * Where no single file stands out, this reports the ambiguity rather than guessing. Picking by
 * something arbitrary like the lowest number would risk showing one proposal's summary and
 * number against a different proposal's vote, which is worse than showing nothing.
 */
export function pickProposalFile(
  files: PullRequestFile[],
  config: ProposalRepoConfig,
): PickProposalFileResult {
  const candidates = files
    .filter((file) => file.status !== "removed")
    .map((file) => {
      const fileName = basename(file.filename);
      const ref = proposalRefFromFileName(fileName, config);
      if (!ref) return undefined;
      if (
        config.proposalDir !== undefined &&
        dirname(file.filename) !== config.proposalDir
      ) {
        return undefined;
      }
      const headSha = headShaFromContentsUrl(file.contents_url);
      if (!headSha) return undefined;
      return { file, path: file.filename, headSha, ref };
    })
    .filter((candidate) => candidate !== undefined);

  if (candidates.length === 0) return { status: "none" };

  // A PR that adds one proposal while editing others — a cross-reference, say — is not
  // ambiguous: the added file is its subject. Two files at equal standing genuinely are.
  const added = candidates.filter(
    (candidate) => candidate.file.status === "added",
  );
  const contenders = added.length > 0 ? added : candidates;

  if (contenders.length > 1) {
    return {
      status: "ambiguous",
      paths: contenders.map((candidate) => candidate.path).sort(),
    };
  }

  const { path, headSha, ref } = contenders[0];
  return { status: "ok", file: { path, headSha, ref } };
}

/**
 * A pull request's files each carry the head SHA in their `contents_url` query string, so it
 * comes free with the file listing — no second API call to `/pulls/{n}` needed.
 */
export function headShaFromContentsUrl(
  contentsUrl: string,
): string | undefined {
  try {
    return new URL(contentsUrl).searchParams.get("ref") ?? undefined;
  } catch {
    return undefined;
  }
}

function basename(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1] ?? "";
}

function dirname(path: string): string {
  const index = path.lastIndexOf("/");
  return index === -1 ? "" : path.slice(0, index);
}
