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

Build only (push image without releasing):

```bash
flyctl deploy --build-only --push -a <app-name> --dockerfile deploy/Dockerfile.fly
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
- If you want CI-driven deploys, I can add a GitHub Actions workflow that runs `flyctl deploy` on push to `main`.
- If you prefer not to use Docker, you can instead build a release binary and deploy to a VPS (we removed that flow at your request).
