# Fork-owned availability-oracle image

Outcome: Both vote paths now use the RPC gas price plus 20%; all 11 tests pass.
Published and pulled the verified Linux AMD64 image `1.4.0-gasprice.1`; GHCR authentication is required.

## Goal
Set explicit RPC gas prices with a 20% buffer for RewardsManager `set_denied_many`
and SubgraphAvailabilityManager `vote_many`, matching DataEdge. Verify the signed
transactions use these fees, then build, smoke-test, and deliver a Linux AMD64
image under `ghcr.io/kw1knode/availability-oracle`.

## Constraints
- Do not alter existing tests.
- Do not push Git branches or tags without explicit permission.
- Do not publish to the upstream `graphprotocol` namespace.
- Limit runtime changes to explicit gas pricing at the two confirmed call sites.
- Preserve the existing 20% gas-limit buffer and contract voting behavior.
- Validate the image with `--version` and `--help`; do not use production signing
  credentials or send blockchain transactions.
- Do not scaffold a project harness without confirmation.

## Tasks
- [x] Compare suntzu93's public fork with the local checkout.
- [x] Run the existing oracle tests before editing.
- [x] Correct and validate the container publishing workflow for this fork.
- [x] Identify the requested source fix and record its scope before implementation.
- [x] Reproduce the source issue, apply the confirmed fix, and verify it.
- [x] Build and smoke-test a Linux container image containing the fix.
- [x] Publish or deliver the verified image and document its exact reference.

## Decisions
2026-09-25 — Work in the user's clean Desktop checkout, as requested.
2026-09-25 — Use the repository owner's GHCR namespace and explicit package-write
permissions so the publishing workflow works in this fork.
2026-09-25 — Do not infer a source patch from the fork name: suntzu93's only branch
is identical to local commit `1fad4b4f092cd85a79d012452a9935ffab4e822f`.

## Log
2026-09-25 — Confirmed the local repository is `kw1knode/subgraph-oracle`, clean on
`main`. There is no root AGENTS.md; offered harness setup without creating it.
2026-09-25 — Baseline `cargo test --locked -p availability-oracle` passed all 7 tests.
2026-09-25 — Found upstream PR #43 by tmigone, which handles IPFS HTTP
401/403/410/451 per deployment. Asked the user whether this is the intended fix.
2026-09-25 — Docker CLI is installed, but the daemon is unavailable. Docker Desktop
startup and UI inspection have not yet succeeded.
2026-09-25 — Updated the image workflow with the repository owner's namespace,
package-write permissions, a manual trigger, source labels, and current pinned
checkout/build/login actions. Kept the Linux AMD64 platform used by upstream.
Excluded Git metadata from the Docker build context.
2026-09-25 — The image workflow passed actionlint v1.7.12; `git diff --check` passed.
Docker Desktop subsequently started, and `docker info` plus Buildx inspection
confirmed the engine and Linux AMD64 builder are available.
2026-09-25 — Waiting for the user to identify the source patch or confirm upstream
PR #43. No source fix, final image build, registry push, or Git push has occurred.
2026-09-25 — User identified the desired change: explicitly fetch RPC gas price,
add a 20% buffer, and set it on both vote transaction builders, following
`data_edge.rs`. PR #43 is outside this task.
2026-09-25 — Plan for regression coverage: run both real contract submission paths
against a local JSON-RPC fixture and inspect the signed transaction fee fields.
The test must fail if either path falls back to ethers' default fee estimator.
2026-09-25 — Added four new local JSON-RPC regression tests without changing
existing tests. Before the fix, both submission paths signed with a fee cap of
3,002,000,000 wei rather than the expected 1,200,000 wei and submitted despite the
fixture's gas-price RPC failure. All four tests failed for the expected behavior.
2026-09-25 — Applied explicit RPC gas pricing plus the 20% buffer at both contract
call sites. All 11 oracle tests now pass, including fresh gas-price lookup for
each 100-deployment batch and no submission when the gas-price RPC fails.
`cargo fmt --all -- --check` and `git diff --check` pass.
2026-09-25 — Use the distinct image tag `1.4.0-gasprice.1` to distinguish the patched
image from the upstream 1.4.0 release.
2026-09-25 — Independent read-only review found no issues requiring changes. It
confirmed the fee setters fill both EIP-1559 fields and bypass the default
estimator. Image build, startup checks, and publication remain outstanding.
2026-09-25 — Linux AMD64 release build completed successfully. Image ID:
`sha256:53d680f2edf9c7f7ec9be71ce8c576ca48f78a17160dd87b8df9ff0e9cd55828`.
The container runs as UID 1000; `--version` and `--help` both exit successfully
with networking disabled. The embedded contract source SHA-256 matches the tested
local file: `5b1fc6642a4aab291ac23e77711637c76b375f881a49dfec2e53a027e1b69a34`.
2026-09-25 — Publishing the image under the new `1.4.0-gasprice.1` tag. The tag did
not previously exist. Source changes remain uncommitted and have not been pushed.
2026-09-25 — Published and successfully pulled
`ghcr.io/kw1knode/availability-oracle:1.4.0-gasprice.1`.
Registry manifest digest:
`sha256:726fc389a7cf49c025fc0718f3de962db9ee4ef0432b7be1ad11306abddae253`.
Anonymous access returns HTTP 401, so deployments need GHCR authentication.
No Git push, source commit, or production deployment was performed.
2026-09-25 — User explicitly requested pushing the changes to the fork. Fresh
`cargo test --locked --workspace` passed 11 tests across 5 suites; formatting,
workflow actionlint, and patch whitespace checks also passed. Remote `main` still
matches the original base commit, so a normal fast-forward push is appropriate.
2026-09-25 — Committed the gas-price fix and regression tests as `f6d2165`. The
image publishing workflow and this record are a separate commit for the same push.
