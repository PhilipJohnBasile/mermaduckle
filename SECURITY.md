# Security Policy

## Reporting

If you find a security issue, report it privately to `pbasile@basilecom.com`.

Do not open a public GitHub issue for credential exposure, auth bypass, or data-access vulnerabilities.

## Repository Boundaries

This repository is public. It is intended to contain:

- application source code
- marketing site source
- demo code and sample data
- deployment manifests with no live secrets
- `.env.example` and setup documentation

This repository must not contain:

- real `.env` files
- live API keys, access tokens, or passwords
- real database connection strings
- customer data, backups, or exports
- production-only admin tooling with embedded credentials

## Secret Handling

Use secret managers and platform secret stores, not Git, for live credentials.

Recommended handling:

- Local development: untracked `.env`
- GitHub Actions: repository or environment secrets
- Fly.io: `fly secrets set`
- Railway: project environment variables
- Shared team storage: 1Password, Bitwarden, Doppler, or another managed secret store

If you need versioned infrastructure secrets, store encrypted files only, using a tool such as `sops` with `age` or your cloud KMS. Do not use a separate private Git repository as a plain-text secret vault.

## If A Secret Is Exposed

1. Rotate the exposed secret immediately at the provider.
2. Replace the value in the relevant platform secret store.
3. Remove any local copies that should not persist.
4. If the secret entered git history, rewrite history and invalidate the old credential.
5. Review logs and access records if the exposed credential had production scope.
