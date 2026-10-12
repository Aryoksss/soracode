# Cloud attack methodology

You have credentials, a token, or an SSRF foothold into a cloud account. This
pack enumerates what they reach and escalates toward the control plane.

## Before you touch anything

Record the key as a `cred` with its kind (`aws_key`, `gcp_token`, `azure_sp`)
and the account/tenant id. Identity is the first thing to establish: every
action below is useless until you know *who* you are.

## 1. Establish identity

- AWS `aws sts get-caller-identity`, then `aws iam get-user` /
  `list-attached-user-policies` and `list-user-policies`.
- GCP `gcloud auth list`, `gcloud projects list`.
- Azure `az account show`, `az ad signed-in-user show`.

## 2. Enumerate permissions

- AWS: `aws iam list-roles`, `list-policies`; check for `iam:PassRole`,
  `sts:AssumeRole`, `lambda:CreateFunction`, and `*:*` wildcards.
- GCP: `gcloud projects get-iam-policy <p>`.
- Azure: `az role assignment list --all`.

## 3. Escalation paths (AWS, ordered by yield)

1. `iam:CreateAccessKey` on another user.
2. `iam:AttachUserPolicy`/`PutUserPolicy` — attach `AdministratorAccess`.
3. `iam:PassRole` + `lambda:CreateFunction`/`InvokeFunction` → run as a role.
4. `sts:AssumeRole` into a role whose trust you satisfy.
5. SSM `SendCommand` into a running instance you can reach.

## 4. Storage and data

- `aws s3 ls`, `aws s3 cp s3://bucket - --recursive` for secrets in objects.
- Check for public buckets, exposed snapshots, and IMDS credentials on any
  instance you reach.

## 5. Proving it

The proof is the API response showing the privilege or the data — an
`Account`, a policy document, or an object body. Record the finding as a `vuln`
with the exact CLI invocation and its output.
