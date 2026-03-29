# Cloudflare Worker Manual Testcase for Issue #18

This fixture exercises the same runtime shape that failed in `mdorigin`:

- `indexbind/cloudflare`
- a virtual bundle base URL such as `https://mdorigin-search.invalid/...`
- `globalThis.fetch` temporarily redirected to `ASSETS.fetch(...)`
- canonical bundle files served from Cloudflare Workers Assets

This fixture already includes a tiny hashing-backed canonical bundle under:

- `public/search/index.bundle`

So it can be deployed as-is.

## Refresh the bundle assets

From the repository root:

```bash
npm run testcase:cloudflare-worker:prepare
```

That refreshes:

- `fixtures/manual/cloudflare-worker-issue-18/public/search/index.bundle`

## Run locally

```bash
npm run testcase:cloudflare-worker:dev
```

Then hit:

```bash
curl 'http://127.0.0.1:8787/api/search?q=rust%20guide'
```

Expected top hit:

- `guides/rust.md`

## Deploy to Cloudflare

```bash
cd fixtures/manual/cloudflare-worker-issue-18
wrangler deploy
```

After deploy:

```bash
curl 'https://<your-worker-host>/api/search?q=rust%20guide'
```

You can also compare against the direct same-origin bundle path:

```bash
curl 'https://<your-worker-host>/api/search?q=rust%20guide&mode=direct'
```

If the Worker still fails to bootstrap wasm, the JSON response includes the original stack instead of only flattening to `Invalid URL string`.
