# Kerberos attack methodology

A Windows domain is in scope and Kerberos (88/tcp) is reachable. This pack
covers the ticket attacks that turn a foothold into domain access.

## Before you touch anything

You need the domain name and at least a username (valid creds are better).
Record every ticket, hash and session with `state_update` as you go — the
`ad-attack` pack has the graph discipline; this pack is the Kerberos specifics.

## 1. Enumeration without credentials

- User enumeration via `kerbrute userenum` or AS-REQ error oracle
  (`KRB_AP_ERR_SKEW` vs `KDC_ERR_C_PRINCIPAL_UNKNOWN`).
- Password spray with `kerbrute passwordspray`, respecting lockout policy.

## 2. AS-REP roasting (no pre-auth)

Accounts with `DONT_REQUIRE_PREAUTH` answer an AS-REQ with a ticket encrypted
to the user's key. Request it and crack offline:

```
GetNPUsers.py <domain>/ -usersfile users.txt -no-pass -format hashcat
hashcat -m 18200 asrep.txt wordlist.txt
```

`nxc ldap <dc> --asreproast out.txt` does it in one step.

## 3. Kerberoasting (SPN accounts)

Any authenticated user can request a service ticket for an SPN; the ticket is
encrypted to the service account's key.

```
GetUserSPNs.py <domain>/<user>:<pass> -request -outputfile tgs.txt
hashcat -m 13100 tgs.txt wordlist.txt
```

Request RC4 (etype 23) for speed. Service accounts are often over-privileged.

## 4. Ticket reuse

- **Pass-the-ticket**: set `KRB5CCNAME` to a `.ccache` and use tools that read
  it (`nxc smb <target> --use-kcache`).
- **Silver ticket**: forge a service ticket with a cracked service key (TGS).
- **Golden ticket**: forge a TGT with the `krbtgt` key (needs DCSync).

## 5. Delegation and escalation

- Unconstrained delegation: compromise a host and steal its TGTs.
- Constrained delegation: `getST.py -impersonate administrator` for the SPN
  the account may delegate to.
- RBCD: write `msDS-AllowedToActOnBehalfOfOtherIdentity`, then S4U.
- DCSync (`secretsdump.py -just-dc`) once you hold replication rights — this is
  usually "game over"; record the extracted `krbtgt` hash as a `cred`.

## 6. Proving it

The proof is a successful authentication or an extracted secret. A crackable
ticket alone is a finding only once the plaintext (or a domain-wide impact such
as a golden ticket) is shown.
