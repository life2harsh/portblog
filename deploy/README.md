# deploy

## blog api (`apps/api`)

Runs on the VM as `life2harsh-blog-api.service`, bound to `127.0.0.1:7841`.

- binary: `/home/ubuntu/blog-api/blog-api` (static aarch64 musl build)
- config: `/home/ubuntu/blog-api/blog-api.env` (mode 600, holds `BLOG_API_TOKEN`)
- data: `/home/ubuntu/blog-api/data/blog.db` (SQLite, WAL)
- limits: 64 MB memory, 25% CPU, read-only filesystem except `data/`

Build:

```sh
RUSTUP_TOOLCHAIN=stable cargo zigbuild -p blog-api --release --target aarch64-unknown-linux-musl
```

Update the binary:

```sh
scp target/aarch64-unknown-linux-musl/release/blog-api ubuntu@VM:/home/ubuntu/blog-api/blog-api.new
ssh ubuntu@VM 'mv ~/blog-api/blog-api.new ~/blog-api/blog-api && sudo systemctl restart life2harsh-blog-api'
```

API:

| method | path | auth | |
|---|---|---|---|
| GET | `/health` | | `ok` |
| GET | `/entries?kind=note\|essay&drafts=true` | drafts only | newest first |
| GET | `/entries/{slug}` | drafts only | |
| POST | `/entries` | yes | `{kind, title, body, tags, published}` |
| PUT | `/entries/{slug}` | yes | partial update |
| DELETE | `/entries/{slug}` | yes | |

Auth is `Authorization: Bearer <BLOG_API_TOKEN>`.

### exposing it

Nothing public points at it yet. Either:

1. **nginx on the VM**: add `blog-api/nginx-blog-api.conf` as a site, point a DNS record at the VM, issue a certificate, then build the blog with `BLOG_API=https://blog-api.life2harsh.xyz`.
2. **Vercel rewrite** (keeps the blog same-origin, no CORS): the blog defaults to calling `/api`, so add to the blog's `vercel.json`:

```json
{
  "rewrites": [
    { "source": "/api/:path*", "destination": "https://blog-api.life2harsh.xyz/:path*" },
    { "source": "/(.*)", "destination": "/index.html" }
  ]
}
```

`BLOG_API_ORIGINS` already allows `https://blog.life2harsh.xyz` and `https://life2harsh.xyz`.

## ssh portfolio (`apps/ssh`)

Not deployed. `ssh-portfolio/life2harsh-ssh.service` runs it on port 2222 next to the blog api, reading the journal from `127.0.0.1:7841`. Port 22 belongs to the VM's own sshd, so either open 2222 (`ssh -p 2222 life2harsh.xyz`) or move the admin sshd before giving 22 to the portfolio.

```sh
RUSTUP_TOOLCHAIN=stable cargo zigbuild -p ssh-portfolio --release --target aarch64-unknown-linux-musl
```

Env: `SSH_BIND`, `SSH_HOST_KEY` (generated on first start, keep it so the fingerprint stays stable), `SSH_JOURNAL`, `SSH_MAX_SESSIONS`.
