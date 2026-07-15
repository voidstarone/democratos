# Democratos — Media Safety

> How uploaded media is kept from being **malicious** (files that attack the
> server or other users) and from carrying **illegal content** (CSAM), and what an
> operator must do to make the CSAM protection real rather than a stub.

Media in Democratos is first-party: an upload is sanitized, scanned, and stored
under a content-addressed key, and it exists only as part of a post. The store a
CDN serves from therefore only ever holds bytes our own encoder produced and our
scanner cleared.

## 1. The ingest pipeline

Every upload passes through `GuardedMediaStore` (a `MediaStore` decorator in the
`app` crate) before it reaches the real backend — local disk, an S3/MinIO bucket a
separate CDN service reads from, or the in-RAM store. The stages, in order:

1. **Sanitize** (`MediaSanitizer`). The default `ImageReencodeSanitizer`:
   - reads image dimensions from the header and rejects a decompression / pixel
     bomb *before* decoding a single pixel (caps: `MAX_IMAGE_PIXELS` = 40 MP,
     `MAX_IMAGE_DIMENSION` = 20 000 px);
   - decodes with hard allocation limits, so a malformed file can't exhaust memory;
   - **re-encodes** PNG and JPEG from the decoded pixels, discarding all original
     metadata (EXIF/GPS), trailing payloads, and polyglot framing;
   - validates animated GIF/WebP by decoding but keeps their bytes (re-encoding
     would flatten the animation) — the bomb/format checks still apply;
   - structurally validates video (mp4/webm) without transcoding (a full re-encode
     needs ffmpeg, too heavy for the small boxes this targets), confirming the
     bytes really carry the container magic they claim.
2. **Scan** (`MediaSafetyScanner`). The default `HashListSafetyScanner` matches the
   sanitized bytes against a curated known-bad corpus — a cryptographic SHA-256
   match (exact copy) and, for images, a perceptual dHash match (resized /
   recompressed copy).
3. **Store or block**:
   - clear → persisted under its canonical content type;
   - a positive match → **blocked and preserved** in quarantine (never stored),
     always, regardless of the failure policy;
   - the scanner can't decide → the per-node **scan-failure policy** applies.

This is the only path uploads take, so the safety posture is assembled in exactly
one place (`build_media_guard` in the composition root) and can't be bypassed by a
new caller.

## 2. Configuration (per node)

| Flag | Env | Default | Meaning |
|------|-----|---------|---------|
| `--media-sanitizer` | `DEMOCRATOS_MEDIA_SANITIZER` | `reencode` | `reencode` (re-encode images) or `passthrough` (type-check only; lighter CPU, weaker) |
| `--csam-scan` | `DEMOCRATOS_CSAM_SCAN` | `false` | Run the CSAM scan at all (off by default — see §3) |
| `--csam-hash-file` | `DEMOCRATOS_CSAM_HASH_FILE` | *(none)* | Path to the known-bad hash corpus |
| `--media-scan-policy` | `DEMOCRATOS_MEDIA_SCAN_POLICY` | `fail-closed` | `fail-closed` \| `quarantine` \| `allow` when the scanner is unavailable |
| `--quarantine-dir` | `DEMOCRATOS_QUARANTINE_DIR` | `quarantine` | Where blocked/held media is preserved |

**Scan-failure policy** governs only the *unavailable* case (a positive match is
always blocked):

- `fail-closed` — refuse the upload. Nothing unscanned is stored. The safe default.
- `quarantine` — refuse *and* preserve a copy for review; never serve it.
- `allow` — serve it unscanned, logging the failure. Highest availability, weakest
  guarantee.

## 3. ⚠️ CSAM scanning is OFF by default

**The CSAM scan is disabled (`--csam-scan false`) unless you explicitly enable it,
and it cannot work without a hash source you may not be able to obtain.** This is a
deliberate honesty decision: a scanner with an empty corpus clears every upload,
which is a *false sense of protection* worse than openly having none. So by default
the node sanitizes uploads (§1) but does not claim to scan them, and says so at
boot. Malicious-media sanitization is unaffected — it always runs.

To turn scanning on you need a real source, because there is no way to build a
working CSAM detector from local heuristics. Effective detection requires one of:

- a **curated hash corpus** from a lawful source — NCMEC / PhotoDNA / IWF hash
  sets, access to which is legally gated to vetted providers — converted into the
  hash-file format below; and/or
- an **external classifier** for *novel* material (Thorn Safer, Google Content
  Safety, Cloudflare's CSAM Scanning Tool). Wire one by implementing
  `app::MediaSafetyScanner` in a new adapter and swapping it in at
  `build_media_guard`. Because that adapter *can* be unavailable, the
  fail-closed / quarantine policy exists for exactly it.

### Hash-file format

Plain text, one entry per line; `#` starts a comment. The file holds only opaque
hashes — never any imagery — so it is safe to store and ship.

```
# exact (cryptographic) match — a byte-identical copy
sha256:<64 hex chars>
# perceptual (dHash) match — a resized/recompressed copy (matched within a few bits)
dhash:<16 hex chars>
```

## 4. Quarantine & the legal duty — do not delete

`DirQuarantine` writes blocked bytes to `--quarantine-dir` with owner-only
permissions and appends an incident record to `incidents.log`. **This is
preservation, not a bin.** In the United States a provider that becomes aware of
apparent CSAM must report it to the NCMEC CyberTipline and **preserve** the content
(18 U.S.C. §2258A); deleting it destroys evidence the law requires be kept. So the
pipeline never discards a blocked upload.

Operator obligations this code does **not** and cannot perform for you:

- Put the quarantine directory on storage only trusted staff can reach; treat
  access as an incident in itself.
- File CyberTipline reports and preserve for the statutory window.
- Restrict who can see `incidents.log` and the held files.

## 5. Serving (the CDN read path)

`GET /media/:key` (used when the store proxies bytes; a public bucket / CDN serves
its own URLs) sets, in addition to the correct `Content-Type`:
`X-Content-Type-Options: nosniff`, `Content-Disposition: inline`,
`Content-Security-Policy: default-src 'none'; sandbox; frame-ancestors 'none'`,
`Cross-Origin-Resource-Policy: same-site`, and a one-year immutable `Cache-Control`
(safe because keys are content-addressed). Together these stop a stored file from
ever being treated as an active document even if its bytes were somehow coaxed
toward one.

## 6. Known residual gap — media whose URL is set directly

The pipeline guards the **upload** path only: `POST /posts` with a `file` part →
`media.put` → the guard. A `Media` whose `url` is set *directly*, without going
through `put`, never touches sanitization or scanning. Two such paths exist today:

- the composer's dragged-in **`media_url`** field (an `http(s)` link the browser
  embeds from a third-party host); and
- programmatically constructed media — e.g. the dev **seed** fixtures embed
  `data:image/svg+xml` URIs directly (`seed/generate_image.rs`).

Such media is not uploaded, not stored, and not scanned by us, so it is outside the
guarantees above. This is also inconsistent with the "media is hosted and tied to a
post" model. (Note: the page renders images with `<img>`, in which browsers do not
execute SVG scripts, so the seed's SVG data URIs are not an XSS vector — but the
*scanning* bypass is real for any imagery embedded this way.)

Options for closing it (a product decision, not yet taken):

- disallow externally-hosted / `data:` media and require an upload (which is then
  sanitized + scanned); or
- server-side fetch the URL, run its bytes through the pipeline, and re-host it
  (adds an SSRF surface that must itself be constrained).

Until then, only uploaded media enjoys the safety guarantees described here.
