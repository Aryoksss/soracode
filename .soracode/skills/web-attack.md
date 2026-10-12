# Web application attack methodology

You have one or more HTTP services in scope. This pack is the path from "a
URL" to "an exploitable finding with proof".

## Before you touch anything

Read the engagement state: `state_query` with `services` and `vulns`. Every
host you probe goes in with `state_update` (`host`); every confirmed issue with
a `vuln` carrying severity and the raw evidence (request/response pair).

## 1. Fingerprint and map

- Stack: response `Server`, `X-Powered-By`, cookies (`PHPSESSID`, `JSESSIONID`,
  `csrftoken`), error page style, `robots.txt`, `sitemap.xml`.
- Enumerate content with a wordlist; parse the result with the matching tool
  (`gobuster_parse`) so directories are recorded, not lost in scrollback.
- Pull every route from client JS: `curl <bundle>.js | rg -o '"/[a-z0-9/_-]+"'`.
  SPAs leak their API surface this way.

## 2. The core test order

1. **Authn/authz** — can you reach an authenticated endpoint unauthenticated
   (IDOR, forced browsing)? Change an object id and see if you get someone
   else's data. This is the highest-yield bug class; check it before anything
   clever.
2. **Injection** — SQLi (`sqlmap_parse` for output), command injection,
   SSTI (`{{7*7}}` → `49`), LDAP, NoSQL. Confirm with time-based or
   boolean-based proof, never a single error string.
3. **XSS** — reflected first, then stored, then DOM. Route reflections through
   `xss_detect` to classify context and payload survival.
4. **File handling** — path traversal (`../../etc/passwd`), unrestricted upload
   (extension, content-type, magic bytes), LFI/RFI.
5. **SSRF** — the target fetches a URL you supply. Load the `ssrf` pack.

## 3. Proving it

A finding needs a reproducible request and the observed response. Capture the
exact request (method, path, headers, body) and the response that proves the
impact — leaked data, a redirect to your host, a callback in your listener.
Write it into `vuln.evidence`. An error message alone is a lead, not a finding.
