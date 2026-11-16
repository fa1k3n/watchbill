<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo-dark.svg">
    <img src="docs/logo-light.svg" width="96" height="96" alt="The mark of Watchbill: a grid of three by three rounded squares, the diagonal filled in another colour">
  </picture>
</p>

# Watchbill

A harness for AI agents in embedded software development, deployed at a fixed revision onto a project that already exists.

**Status: an idea, written down 2026-11-16. Nothing is built.**

## What it is to be

- **Deployed onto what the project already has.** The requirements tool, the tracker, the git server and the CI driver stay as they are.
- **The agents show up where the team works.** When Watchbill is deployed, the agents appear in the team's own tools, not in a new place to look.
- **One fixed revision.** A deployment is one revision of the harness, and the prompts are under version control.
- **Traceable.** What an agent did, for which requirement, under which revision, can be followed afterwards.
- **A method that can be certified.** The aim is that a team's process with Watchbill in it can pass an assessment against an ISO standard. Watchbill is not certified, and nothing here says it is.

## The name

A watch bill is a ship's posted list of who stands which watch at which station.

## Beside Handover Ledger

Watchbill is a product of its own. [Handover Ledger](https://github.com/fa1k3n/handover-ledger) records the handovers in a team and can be used to monitor a team that runs Watchbill. Watchbill does not need it.

## Proposed, not decided

- **The first piece is the manifest:** one file with the hashes of the state program, the role prompts, the rule table and the model IDs, which together give the revision. Every assignment names it.
- **The safety argument rests on deterministic code and a human verifier.** A model cannot be qualified as a tool, so a model's output is never the last check. A changed prompt or model is a new revision.
- **Agents are accounts** at the git server and the tracker, on runners registered the way CI runners are.
- **Adapters** carry the differences between requirements tools, trackers, git servers and CI drivers.

## Not decided

- The licence.
- Which standard comes first, and which tools the first adapters are for.
- The language it is written in.
