//! Validation for the `--description` GitHub link on `create-proposal`.
//!
//! The on-chain program (`svmgov_program::utils::is_valid_github_link`) only checks that the
//! description looks broadly like a GitHub URL: an `https://github.com/` prefix, 2-10 path
//! segments, and characters limited to alphanumerics plus `-`, `_`, `.`. A pull request link
//! such as `.../owner/repo/pull/3` is four clean segments, so it sails through — and then the
//! frontend cannot resolve it to a proposal document.
//!
//! This module closes that gap client-side. It does NOT reimplement the on-chain rules; it is
//! strictly stricter by construction, accepting only
//! `https://github.com/{owner}/{repo}/blob/{ref}/{...}.md` and then re-checking the on-chain
//! grammar directly, so anything accepted here is guaranteed to be accepted on chain.
//!
//! Rule numbering matches frontend/src/lib/github/validateProposalUrl.ts; keep the two in step.

use std::time::Duration;

use anyhow::{Result, anyhow};

/// The exact prefix `is_valid_github_link` requires. A `www.` host or a
/// `raw.githubusercontent.com` link would be rejected on chain, so they are rejected here.
const GITHUB_PREFIX: &str = "https://github.com/";

/// Mirrors `svmgov_program::utils::is_valid_github_link`.
const MIN_PATH_SEGMENTS: usize = 2;
const MAX_PATH_SEGMENTS: usize = 10;

/// Soft mirror of `global_config.max_description_length`. That value is configurable on chain,
/// so this is a client-side courtesy check rather than the authority.
const MAX_DESCRIPTION_BYTES: usize = 500;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

const PULL_REQUEST_HELP: &str = "\
Open the pull request's \"Files changed\" tab, click the proposal .md file, and copy its URL:

  https://github.com/<owner>/<repo>/blob/<40-char-commit-sha>/proposals/sgp-0001-title.md

Prefer the commit SHA over a branch name. The description is stored on chain and cannot be
edited, so a branch link breaks as soon as the branch moves or is deleted.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubLinkKind {
    Blob {
        owner: String,
        repo: String,
        git_ref: String,
        path: String,
    },
    Pull {
        owner: String,
        repo: String,
        number: u64,
    },
    /// A directory listing (`/tree/`).
    Tree,
    Other,
}

/// Classifies a GitHub URL by shape. Never fails; anything unrecognized is `Other`.
pub fn classify_github_link(link: &str) -> GithubLinkKind {
    let Some(path) = link.trim().strip_prefix(GITHUB_PREFIX) else {
        return GithubLinkKind::Other;
    };

    // Classification ignores a query string or fragment; rule 7 rejects them separately.
    let path = path
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 3 {
        return GithubLinkKind::Other;
    }

    let (owner, repo, kind) = (segments[0], segments[1], segments[2]);

    match kind {
        "blob" | "raw" if segments.len() >= 5 => GithubLinkKind::Blob {
            owner: owner.to_string(),
            repo: repo.to_string(),
            git_ref: segments[3].to_string(),
            path: segments[4..].join("/"),
        },
        "tree" => GithubLinkKind::Tree,
        "pull" | "pulls" => match segments.get(3).and_then(|n| n.parse::<u64>().ok()) {
            Some(number) => GithubLinkKind::Pull {
                owner: owner.to_string(),
                repo: repo.to_string(),
                number,
            },
            None => GithubLinkKind::Other,
        },
        _ => GithubLinkKind::Other,
    }
}

/// Applies rules 1-11 with no network access. Warnings (9-11) go to `log::warn!`.
pub fn validate_description_structure(description: &str) -> Result<GithubLinkKind> {
    let link = description.trim();

    // 1
    if link.is_empty() {
        return Err(anyhow!(
            "`--description` must be a GitHub link to the proposal markdown file"
        ));
    }

    // 8 — cheap, and it bounds everything below.
    if link.len() > MAX_DESCRIPTION_BYTES {
        return Err(anyhow!(
            "`--description` is {} bytes; the on-chain limit is {MAX_DESCRIPTION_BYTES}",
            link.len()
        ));
    }

    // 2
    if link.starts_with("http://") {
        return Err(anyhow!(
            "`--description` must use https, not http\n\n  got: {link}"
        ));
    }

    // 3 — the on-chain check requires this exact prefix, so `www.github.com` and
    // `raw.githubusercontent.com` links are rejected on chain even though they resolve.
    if !link.starts_with(GITHUB_PREFIX) {
        return Err(anyhow!(
            "`--description` must start with {GITHUB_PREFIX} (no `www.`, and not raw.githubusercontent.com)\n\n  got: {link}"
        ));
    }

    // 7 — `is_valid_github_link` rejects these outright, so they would fail on chain.
    if link.contains(['?', '#']) {
        return Err(anyhow!(
            "`--description` must not contain a query string or #fragment; the on-chain program rejects them\n\n  got: {link}"
        ));
    }

    let kind = classify_github_link(link);

    // 4
    if let GithubLinkKind::Pull { number, .. } = &kind {
        return Err(anyhow!(
            "`--description` must link to the proposal markdown file, not to pull request #{number}.\n\n  got: {link}\n\n{PULL_REQUEST_HELP}"
        ));
    }

    // 5
    if matches!(kind, GithubLinkKind::Tree) {
        return Err(anyhow!(
            "`--description` links to a directory listing. Link to the proposal markdown file itself.\n\n  got: {link}"
        ));
    }

    let GithubLinkKind::Blob {
        owner,
        repo,
        git_ref,
        path,
    } = &kind
    else {
        return Err(anyhow!(
            "`--description` must be a link to a file on GitHub, e.g.\n\n  https://github.com/<owner>/<repo>/blob/<commit-sha>/proposals/sgp-0001-title.md\n\n  got: {link}"
        ));
    };

    // 6
    let file_name = path.rsplit('/').next().unwrap_or_default();
    if !file_name.to_ascii_lowercase().ends_with(".md") {
        return Err(anyhow!(
            "`--description` must link to a .md file\n\n  got: {link}"
        ));
    }

    // Re-check the on-chain grammar rather than assuming the shape above implies it.
    assert_on_chain_compatible(link)?;

    // 9
    if !is_commit_sha(git_ref) {
        log::warn!(
            "`{git_ref}` is a branch or tag. The description cannot be changed once it is on chain, \
             so a full commit SHA is safer against the branch moving or being deleted."
        );
    }

    // 10 — unknown repos are allowed; only the shape is enforced.
    if !is_known_proposal_repo(owner, repo) {
        log::warn!("{owner}/{repo} is not a recognized proposal repository");
    }

    // 11
    if proposal_number(file_name).is_none() {
        log::warn!(
            "`{file_name}` does not look like a proposal filename (expected `sgp-0001-title.md` \
             or `0001-title.md`), so no proposal number will be shown in the UI"
        );
    }

    Ok(kind)
}

/// Structural validation plus an optional check that the file actually exists.
///
/// Only an authoritative 404 blocks. Transport failures (offline, DNS, TLS, timeout) warn and
/// pass, so a flaky or absent network never stops a proposal from being created.
///
/// Returns the normalized link, which the caller must submit in place of the raw argument:
/// validation trims, and the program requires a literal `https://github.com/` prefix, so a
/// value with surrounding whitespace would be rejected on chain despite passing here.
pub async fn validate_description(description: &str, skip_network: bool) -> Result<String> {
    let kind = validate_description_structure(description)?;
    let normalized = description.trim().to_string();

    if skip_network {
        log::debug!("skipping proposal link reachability check");
        return Ok(normalized);
    }

    let GithubLinkKind::Blob {
        owner,
        repo,
        git_ref,
        path,
    } = kind
    else {
        return Ok(normalized);
    };

    // Checked against the raw URL rather than the HTML page: 200/404 is unambiguous, there is
    // no HTML to parse, it is not subject to the GitHub API rate limit, and it is exactly the
    // URL the frontend will fetch — so a pass here proves the frontend can render it.
    let raw_url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{git_ref}/{path}");
    check_reachable(&raw_url).await?;

    Ok(normalized)
}

async fn check_reachable(raw_url: &str) -> Result<()> {
    // GitHub rejects some requests without a User-Agent, and the default client has no timeout.
    let client = reqwest::Client::builder()
        .user_agent(concat!("svmgov/", env!("CARGO_PKG_VERSION")))
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    let response = match client.head(raw_url).send().await {
        Ok(response) => response,
        Err(e) => return warn_and_continue(e, raw_url),
    };

    // Some intermediaries refuse HEAD; retry as a one-byte GET.
    let response = if matches!(response.status().as_u16(), 405 | 501) {
        match client
            .get(raw_url)
            .header("Range", "bytes=0-0")
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => return warn_and_continue(e, raw_url),
        }
    } else {
        response
    };

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!(
            "proposal file not found at {raw_url}\n\n\
             Check the path and the branch or commit. If the proposal only exists on a pull \
             request, use the blob URL at the PR's head commit rather than the PR link itself.\n\n\
             Re-run with --skip-link-check to create the proposal anyway."
        ));
    }

    log::warn!("could not verify proposal link: GitHub returned {status} for {raw_url}");
    Ok(())
}

fn warn_and_continue(error: reqwest::Error, raw_url: &str) -> Result<()> {
    if error.is_connect() || error.is_timeout() || error.is_request() {
        log::warn!("could not verify {raw_url} (offline?): {error}");
        return Ok(());
    }
    Err(anyhow!("failed to verify {raw_url}: {error}"))
}

/// Proves the claim in this module's docs: anything accepted here is accepted on chain.
fn assert_on_chain_compatible(link: &str) -> Result<()> {
    let path = link.trim_start_matches(GITHUB_PREFIX).trim_end_matches('/');
    let segments: Vec<&str> = path.split('/').collect();

    if segments.iter().any(|segment| segment.is_empty()) {
        return Err(anyhow!(
            "`--description` contains an empty path segment; the on-chain program rejects it\n\n  got: {link}"
        ));
    }

    if !(MIN_PATH_SEGMENTS..=MAX_PATH_SEGMENTS).contains(&segments.len()) {
        return Err(anyhow!(
            "`--description` has {} path segments; the on-chain program accepts {MIN_PATH_SEGMENTS}-{MAX_PATH_SEGMENTS}\n\n  got: {link}",
            segments.len()
        ));
    }

    if let Some(bad) = path
        .chars()
        .find(|c| *c != '/' && !c.is_alphanumeric() && !matches!(c, '-' | '_' | '.'))
    {
        return Err(anyhow!(
            "`--description` contains `{bad}`, which the on-chain program rejects; only letters, \
             digits, `-`, `_` and `.` are allowed in the path\n\n  got: {link}"
        ));
    }

    Ok(())
}

fn is_commit_sha(git_ref: &str) -> bool {
    git_ref.len() == 40 && git_ref.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_known_proposal_repo(owner: &str, repo: &str) -> bool {
    matches!(
        (
            owner.to_ascii_lowercase().as_str(),
            repo.to_ascii_lowercase().as_str()
        ),
        (
            "solana-foundation",
            "solana-improvement-documents" | "solana-governance-proposals"
        )
    )
}

/// `sgp-0001-title.md` -> `0001`, `0022-multi-stake.md` -> `0022`. Leading zeros are kept.
fn proposal_number(file_name: &str) -> Option<String> {
    let stem = file_name
        .strip_suffix(".md")
        .or_else(|| file_name.strip_suffix(".MD"))?;
    let rest = stem
        .strip_prefix("sgp-")
        .or_else(|| stem.strip_prefix("SGP-"))
        .unwrap_or(stem);

    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > 5 {
        return None;
    }

    // The digits must be the whole stem or be followed by a separator, so `007543-x.md` and
    // `0001abc.md` are not mistaken for proposals.
    match rest[digits.len()..].chars().next() {
        None => Some(digits),
        Some('-' | '_' | '.') => Some(digits),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMD_FILE: &str = "https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0022-multi-stake.md";
    const SGP_FILE: &str = "https://github.com/solana-foundation/solana-governance-proposals/blob/27bca51e5c0fc34ddbea6904faf86f5098225316/proposals/sgp-0001-solana-constitution.md";
    const SGP_PULL: &str =
        "https://github.com/solana-foundation/solana-governance-proposals/pull/3";

    #[test]
    fn classifies_blob_links() {
        assert_eq!(
            classify_github_link(SIMD_FILE),
            GithubLinkKind::Blob {
                owner: "solana-foundation".into(),
                repo: "solana-improvement-documents".into(),
                git_ref: "main".into(),
                path: "proposals/0022-multi-stake.md".into(),
            }
        );
    }

    #[test]
    fn classifies_pull_links() {
        for link in [
            SGP_PULL,
            &format!("{SGP_PULL}/files"),
            &format!("{SGP_PULL}/files#diff-abc"),
            &format!("{SGP_PULL}/"),
        ] {
            assert_eq!(
                classify_github_link(link),
                GithubLinkKind::Pull {
                    owner: "solana-foundation".into(),
                    repo: "solana-governance-proposals".into(),
                    number: 3,
                },
                "failed for {link}"
            );
        }
    }

    #[test]
    fn classifies_other_shapes() {
        assert_eq!(
            classify_github_link("https://github.com/org/repo/tree/main/proposals"),
            GithubLinkKind::Tree
        );
        for link in [
            "https://github.com/org/repo",
            "https://github.com/org/repo/issues/12",
            "https://gitlab.com/org/repo/blob/main/x.md",
            "",
        ] {
            assert_eq!(
                classify_github_link(link),
                GithubLinkKind::Other,
                "failed for {link}"
            );
        }
    }

    #[test]
    fn accepts_real_proposal_links() {
        for link in [SIMD_FILE, SGP_FILE] {
            assert!(
                validate_description_structure(link).is_ok(),
                "should accept {link}"
            );
        }
        // Surrounding whitespace is tolerated.
        assert!(validate_description_structure(&format!("  {SGP_FILE}  ")).is_ok());
    }

    #[test]
    fn rejects_pull_request_links_with_guidance() {
        let error = validate_description_structure(SGP_PULL)
            .unwrap_err()
            .to_string();
        assert!(error.contains("pull request #3"), "got: {error}");
        assert!(error.contains("Files changed"), "got: {error}");
        assert!(error.contains("blob/<40-char-commit-sha>"), "got: {error}");
    }

    #[test]
    fn rejects_bad_shapes() {
        let cases = [
            ("", "GitHub link"),
            ("   ", "GitHub link"),
            (
                "http://github.com/o/r/blob/main/proposals/0001-x.md",
                "https",
            ),
            (
                "https://www.github.com/o/r/blob/main/proposals/0001-x.md",
                "must start with",
            ),
            (
                "https://raw.githubusercontent.com/o/r/main/proposals/0001-x.md",
                "must start with",
            ),
            (
                "https://gitlab.com/o/r/blob/main/proposals/0001-x.md",
                "must start with",
            ),
            (
                "https://github.com/o/r/tree/main/proposals",
                "directory listing",
            ),
            ("https://github.com/o/r", "link to a file"),
            ("https://github.com/o/r/issues/12", "link to a file"),
            (
                "https://github.com/o/r/blob/main/proposals/0001-x.txt",
                ".md file",
            ),
            (
                "https://github.com/o/r/blob/main/proposals/0001-x.md?plain=1",
                "query string",
            ),
            (
                "https://github.com/o/r/blob/main/proposals/0001-x.md#L10",
                "query string",
            ),
        ];

        for (link, expected) in cases {
            let error = validate_description_structure(link)
                .unwrap_err()
                .to_string();
            assert!(
                error.contains(expected),
                "for {link:?} expected message containing {expected:?}, got: {error}"
            );
        }
    }

    #[test]
    fn rejects_over_length_descriptions() {
        let link = format!(
            "https://github.com/o/r/blob/main/proposals/{}/0001-x.md",
            "a".repeat(MAX_DESCRIPTION_BYTES)
        );
        let error = validate_description_structure(&link)
            .unwrap_err()
            .to_string();
        assert!(error.contains("on-chain limit"), "got: {error}");
    }

    #[test]
    fn rejects_characters_the_program_rejects() {
        let error = validate_description_structure(
            "https://github.com/o/r/blob/main/proposals/0001%20x.md",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("on-chain program rejects"), "got: {error}");
    }

    #[test]
    fn rejects_paths_deeper_than_the_program_allows() {
        let link = format!(
            "https://github.com/o/r/blob/main/{}/0001-x.md",
            ["deep"; 8].join("/")
        );
        let error = validate_description_structure(&link)
            .unwrap_err()
            .to_string();
        assert!(error.contains("path segments"), "got: {error}");
    }

    /// Everything this module accepts must also satisfy the on-chain
    /// `is_valid_github_link`. Re-derived here so that loosening the rules above without
    /// re-checking the program's grammar fails in CI rather than at transaction time.
    #[test]
    fn accepted_links_satisfy_the_on_chain_rules() {
        let accepted = [
            SIMD_FILE,
            SGP_FILE,
            "https://github.com/o/r/blob/main/x.md",
            "https://github.com/o/r/blob/main/a/b/c/d/e/f.md",
        ];

        for link in accepted {
            assert!(
                validate_description_structure(link).is_ok(),
                "fixture should be accepted: {link}"
            );

            let path = link.strip_prefix(GITHUB_PREFIX).expect("prefix");
            let segments: Vec<&str> = path.split('/').collect();

            assert!(!link.contains([' ', '?', '#']), "{link}");
            assert!(segments.iter().all(|s| !s.is_empty()), "{link}");
            assert!(
                (MIN_PATH_SEGMENTS..=MAX_PATH_SEGMENTS).contains(&segments.len()),
                "{link} has {} segments",
                segments.len()
            );
            assert!(
                path.chars()
                    .all(|c| c == '/' || c.is_alphanumeric() || matches!(c, '-' | '_' | '.')),
                "{link}"
            );
        }
    }

    #[test]
    fn parses_proposal_numbers() {
        assert_eq!(
            proposal_number("0022-multi-stake.md").as_deref(),
            Some("0022")
        );
        assert_eq!(
            proposal_number("00754-token22.md").as_deref(),
            Some("00754")
        );
        assert_eq!(proposal_number("0022.md").as_deref(), Some("0022"));
        assert_eq!(
            proposal_number("sgp-0001-solana-constitution.md").as_deref(),
            Some("0001")
        );
        assert_eq!(proposal_number("XXXX-sgp-template.md"), None);
        assert_eq!(proposal_number("README.md"), None);
        assert_eq!(proposal_number("007543-too-long.md"), None);
        assert_eq!(proposal_number("0001abc.md"), None);
    }

    /// The program requires a literal `https://github.com/` prefix, so submitting the raw
    /// argument after validating its trimmed form would be rejected on chain despite the CLI
    /// having accepted it. Callers must submit what `validate_description` returns.
    #[tokio::test]
    async fn returns_the_trimmed_link_for_submission() {
        let normalized = validate_description(&format!("\n  {SGP_FILE}\t "), true)
            .await
            .expect("should accept a link with surrounding whitespace");
        assert_eq!(normalized, SGP_FILE);
    }

    #[test]
    fn recognizes_commit_shas() {
        assert!(is_commit_sha("27bca51e5c0fc34ddbea6904faf86f5098225316"));
        assert!(!is_commit_sha("main"));
        assert!(!is_commit_sha("27bca51"));
    }

    #[test]
    fn unknown_repos_are_accepted() {
        assert!(
            validate_description_structure(
                "https://github.com/someone/fork/blob/main/proposals/sgp-0002-x.md"
            )
            .is_ok()
        );
        assert!(!is_known_proposal_repo("someone", "fork"));
        assert!(is_known_proposal_repo(
            "solana-foundation",
            "solana-governance-proposals"
        ));
    }
}
