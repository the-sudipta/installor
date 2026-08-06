# Security Policy

## Supported versions

<table width="100%">
  <tr><th width="40%" align="left">Version</th><th width="60%" align="left">Security support</th></tr>
  <tr><td>Latest release</td><td>Supported</td></tr>
  <tr><td>Older releases and development snapshots</td><td>Best effort; upgrade may be required</td></tr>
</table>

## Report a vulnerability privately

Use the repository's **Security → Report a vulnerability** function to open a private GitHub Security Advisory:

`https://github.com/the-sudipta/installor/security/advisories/new`

Include the affected version, operating system, reproduction steps, impact, and any suggested mitigation. Do not open a public issue for an unpatched vulnerability and do not include real credentials, private spreadsheets, or institutional data.

You can expect an initial acknowledgement when maintainer capacity permits. No fixed response or remediation deadline is promised. Coordinated disclosure timing will be discussed based on severity and the availability of a tested release.

## Security boundaries

Installor orchestrates third-party installers and native package managers; it cannot guarantee the safety of software selected by a user. Prefer official HTTPS sources, publisher signatures, package-manager identifiers, and independently obtained checksums. Checksum equality detects a difference from an expected file but does not by itself establish publisher identity.

Unsigned community builds can legitimately trigger SmartScreen or Gatekeeper. Release signing and notarization are separate trust controls and are not claimed for v0.2.0.
