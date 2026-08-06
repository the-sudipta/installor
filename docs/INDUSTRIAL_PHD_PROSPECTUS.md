# Installor: Industrial PhD and Research Collaboration Prospectus

## Executive proposition

Software installation is often treated as a solved engineering task, yet real environments combine heterogeneous package managers, direct installers, incomplete provenance, changing URLs, inconsistent metadata, user confirmation, organizational policy, and offline or partially connected operation. Installor provides a working, inspectable research artifact for studying how deployment automation can become more dependable without becoming opaque or coercive.

The project is suitable as a foundation—not finished evidence—for an Industrial PhD, funded research programme, or industry-academic pilot focused on trustworthy software acquisition and human-centered automation.

## Research problem

Current approaches frequently optimize for either full automation or manual flexibility. The unresolved research space is how to combine:

- portable descriptions of intended software state;
- trustworthy acquisition and provenance evidence;
- heterogeneous platform execution;
- privacy-preserving inventory and synchronization;
- understandable conflict resolution;
- explicit, usable human oversight; and
- empirically measured outcomes rather than assumed safety.

## Existing research artifact

Installor v0.2.0 contributes an executable Tauri/Rust prototype with a local SQLite model, cross-platform detection parsers, account-independent public-Sheet exchange, guided owner-controlled write-back, named-device categorization, integrity-aware downloads, structured install plans, explicit suggestion confirmation, offline retry, automated tests, and documented trust boundaries.

This artifact makes it possible to instrument realistic workflows and compare design alternatives while keeping the implementation small enough for rigorous inspection.

## Candidate research questions

1. How should deployment systems communicate provenance, checksum, signature, and policy evidence so that non-specialists make better decisions?
2. When do human confirmations reduce risk, and when do they become habituated, ineffective interruptions?
3. Which reconciliation models best preserve user intent across local-first, intermittently connected software catalogs?
4. Can package-manager and direct-installer metadata be normalized without hiding platform-specific semantics?
5. How can organizations measure deployment reliability while minimizing collection of sensitive device inventories?
6. What reproducibility guarantees are feasible when upstream installer artifacts, URLs, and package repositories change?

## Work packages

<table width="100%">
  <tr><th width="20%" align="left">Work package</th><th width="45%" align="left">Research activity</th><th width="35%" align="left">Illustrative output</th></tr>
  <tr><td>WP1 — Evidence model</td><td>Systematic review and taxonomy of identity, provenance, integrity, policy, and platform evidence.</td><td>Formal model, threat analysis, annotated corpus.</td></tr>
  <tr><td>WP2 — Human oversight</td><td>Design and evaluate confirmations, explanations, and recovery flows.</td><td>Prototype variants, usability study, design guidance.</td></tr>
  <tr><td>WP3 — Reconciliation</td><td>Compare timestamp, operation-based, and policy-aware synchronization under intermittent connectivity.</td><td>Algorithms, simulation, reproducibility package.</td></tr>
  <tr><td>WP4 — Cross-platform evaluation</td><td>Construct governed testbeds across Windows, macOS, and Linux with representative package routes.</td><td>Benchmark protocol, reliability evidence, failure taxonomy.</td></tr>
  <tr><td>WP5 — Industrial pilot</td><td>Deploy a bounded version with an approved partner and measured objectives.</td><td>Case study, operational guidance, technology-transfer plan.</td></tr>
</table>

## Methodology

A defensible programme could combine design-science research, threat modeling, controlled usability experiments, fault injection, cross-platform benchmark execution, qualitative interviews, and longitudinal pilot observation. Protocols should be preregistered when appropriate. Synthetic catalogs should be preferred until governance and ethics approval permit organizational data.

## Industrial relevance

Potential beneficiaries include IT service teams, laboratories, education providers, software vendors, managed-service providers, cyber-security groups, and organizations operating mixed fleets. Commercial value could arise from reduced setup time, clearer audit evidence, safer self-service, policy-aware catalogs, and reproducible onboarding—claims that should be tested rather than presumed.

## Collaboration sought

- An academic supervisor with expertise in software engineering, HCI, cyber security, distributed systems, or empirical methods.
- An industry partner with a genuine mixed-platform deployment problem and capacity to support a governed pilot.
- Funding for researcher time, test infrastructure, platform signing, participant studies, dissemination, and maintenance.
- Access to domain expertise and non-sensitive workflows; real inventory data is not required for initial research.

## Governance and ethics

Any collaboration should define data minimization, consent, access control, retention, incident handling, intellectual property, open-science outputs, publication review, conflicts of interest, and responsible disclosure. The public repository contains no claim of ethics approval, sponsor commitment, institutional endorsement, or validated organizational benefit.

## Evaluation indicators

Possible measures include task completion, configuration error rate, time to recovery, provenance comprehension, unsafe approval rate, cross-platform consistency, synchronization convergence, privacy exposure, reproducibility, and maintainer effort. Final measures should be negotiated from the research questions and partner context.

## Contact and next step

Prospective supervisors, laboratories, and industry partners can contact [`the-sudipta`](https://github.com/the-sudipta) through the verified GitHub profile with a short description of the research fit, organization, available supervision or funding mechanism, desired timeline, and data-governance constraints. See [FUNDING.md](../FUNDING.md) for engagement models.
