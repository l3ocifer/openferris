# Security Policy

## Supported Versions

OpenFerris is pre-1.0. Security fixes are applied to `main`.

## Reporting a Vulnerability

Please report vulnerabilities privately:
- Email: `security@openferris.com`
- Subject: `[OpenFerris Security] <short title>`

Include:
1. Description of the issue.
2. Reproduction steps or proof-of-concept.
3. Potential impact.
4. Suggested mitigation (if available).

## Response Targets

1. Initial acknowledgment: within **24 hours**.
2. Triage and severity assessment: within **72 hours**.
3. Patch timeline (by severity):
   - Critical: same-day patch.
   - High: within 48 hours.
   - Medium: within 1 week.
   - Low: next scheduled release.
4. Coordinated disclosure: case-by-case based on severity.

## Credit

We credit all reporters in the advisory and release notes (unless
anonymity is requested).

## Scope

In scope:
- `ferris` and all workspace crates.
- `ferris-coordinator` and its API.
- CI workflows and release pipeline.
- Install scripts and published binaries.

Out of scope:
- Third-party services and integrations not controlled by this repository.
