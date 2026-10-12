# Active Directory attack methodology

You already have a foothold on a domain host. This pack is the playbook from
"valid credential" to "domain admin".

## Before you touch anything

Read the engagement state: `state_query` with `creds`, `access` and `edges`.
The graph of who reaches what decides your path. Every credential you obtain
goes back in with `state_update` (`cred`); every session with `access`; every
"X can admin Y" with `edge`.

## 1. Domain enumeration with the foothold

- `nxc smb <dc> -u <user> -p <pass> --users --groups --shares`
- `nxc ldap <dc> -u <user> -p <pass> --query "(objectClass=user)" ""`
- BloodHound collection if the tools are present; parse the JSON and record
  users, computers, sessions and ACLs as state.

Record users as graph nodes so later technique output can be joined to them.

## 2. Credential acquisition in order of cheapness

1. **Kerberoast** — accounts with SPNs. `GetUserSPNs.py` / `nxc ldap --kerberoasting`.
   Request in RC4 (etype 23) for faster cracking. Record each hash as a `cred`
   of kind `hash`.
2. **AS-REP roast** — accounts without pre-auth. `nxc ldap --asreproast`.
   Watch for users an SMB enumeration already revealed.
3. **Password spray** — after you know the policy
   (`nxc smb <dc> --pass-pol`). One password across many users, slow and quiet,
   respecting the lockout threshold. Record hits immediately.
4. **GPP / unattend / LAPS-readable** — check SYSVOL for `cpassword`,
   `unattend.xml`, readable LAPS passwords.
5. **ADCS** — misconfigured templates (`ESC1`–`ESC8`) are the most common
   instant win. Enumerate with `certipy find -vulnerable`.

## 3. Lateral movement

- Pass-the-hash with a captured NTLM hash: `nxc smb <t> -u <u> -H <hash>`.
- WinRM / PsExec / WMI when you hold admin on a target.
- Track direction: only move along edges you can justify. Record the new
  `access` and any `CREDENTIAL_FROM` / `ADMIN_TO` edges.

## 4. Domain takeover

- DCSync when you hold replication rights (`secretsdump.py -just-dc`).
- Delegation abuse (unconstrained / constrained / RBCD).
- ACL abuse: generic all / write DCSync, write owner, add-member to a
  privileged group.

## 5. Report the path

Reconstruct the chain from initial credential to domain admin using the graph:
each hop should name the source, the technique, the evidence, and the resulting
access. That chain is the finding.
