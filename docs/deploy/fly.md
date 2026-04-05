Fly.io Deployment Guide
=======================

This guide shows the minimal steps to deploy Mermaduckle to Fly.io using the included Dockerfile and `fly.toml` configuration.

Prerequisites
- `flyctl` installed and authenticated (`flyctl auth login` or use an access token).
- A Fly app created (either via `flyctl apps create` or `flyctl deploy`).

Build & deploy (recommended)

```bash
# from repo root
flyctl deploy -a <app-name> --dockerfile deploy/Dockerfile.fly
```

This builds the release binary inside the Docker build stage, pushes the image to Fly's registry, and releases the app.

Deploy via the local PowerShell helper

```powershell
pwsh -File tools/deploy_fly.ps1 -AppName mermaduckle -RemoteOnly
```

Use `-BuildOnly` if you want to push the image without releasing it yet.

Build only (push image without releasing):

```bash
flyctl deploy --build-only --push -a <app-name> --dockerfile deploy/Dockerfile.fly
```

Automatic deploys from GitHub Actions

1. Create an app-scoped deploy token for the Fly app:

```bash
fly tokens create deploy -a <app-name>
```

2. Save the token as the GitHub Actions repository secret `FLY_API_TOKEN`.
3. Push to `main` or trigger the workflow manually. The repo includes `.github/workflows/fly-deploy.yml`, which runs:

```bash
flyctl deploy --remote-only --config fly.toml
```

Add a custom domain

1. Add domain to the Fly app:

```bash
flyctl domains add mermaduckle.com -a <app-name>
```

2. Fly will output DNS records you must add at your registrar. For root/apex domains Fly may recommend allocating static IPs:

```bash
flyctl ips allocate-v4 -a <app-name>
# add A records to your registrar pointing to the returned IP(s)
```

3. After DNS propagation, request a certificate (Fly manages Let's Encrypt certificates):

```bash
flyctl certs create mermaduckle.com -a <app-name>
```

Check status

```bash
flyctl apps show <app-name>
flyctl domains list -a <app-name>
flyctl certs list -a <app-name>
```

Notes
- If you prefer not to use Docker, you can instead build a release binary and deploy to a VPS (we removed that flow at your request).
- Fly recommends using the narrowest token scope that works for CI/CD. An app-scoped deploy token is the right default for this repo.
