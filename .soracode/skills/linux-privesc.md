# Linux privilege escalation

You have a low-privilege shell on a Linux host. This pack is the order of
checks that finds the fastest path to root.

## Orient first

- `id`, `hostname`, `uname -a`, `cat /etc/os-release`
- Record the host and what user you are in the engagement state, and any
  credential you used to get here.

## 1. Sudo — check this before anything else

- `sudo -l` — the single highest-yield command. A NOPASSWD entry is often a
  one-step root.
- For each allowed binary, check GTFOBins for a shell or file-read escape.
- Watch for wildcards, relative paths, and env vars you control in the sudoers
  rule.

## 2. SUID / capabilities

- `find / -perm -4000 -type f 2>/dev/null` then cross-reference GTFOBins.
- `getcap -r / 2>/dev/null` — `cap_setuid+ep` on a script interpreter is root.

## 3. Cron and timers

- `cat /etc/crontab`, `ls -la /etc/cron.*`, `systemctl list-timers`
- A world-writable script or a writable directory in a root cron `PATH` is a
  root shell at the next tick.

## 4. Writable service and config files

- Anything root runs that you can write: systemd units, init scripts, logrotate
  rules, library paths (`LD_PRELOAD` via a writable `.conf`).

## 5. Credentials on disk

- `grep -rEi 'password|secret|token' /home /var/www /opt 2>/dev/null`
- Shell history, `.bash_history`, DB configs, `.git/config` remotes.
- SSH keys and `~/.ssh/known_hosts` for lateral movement.

## 6. Kernel and known CVEs

Last resort: match `uname -r` against local-privesc exploits. Prefer a
configuration flaw over a kernel exploit — quieter and more reliable.

## Record it

Every credential, key and foothold goes back into the engagement state
(`state_update`) so the path from user to root is reconstructable for the
report, with the exact command and output as evidence.
