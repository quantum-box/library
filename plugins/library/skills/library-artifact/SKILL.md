---
name: library-artifact
description: Author a self-contained HTML page — a report, dashboard, prototype, diagram page, or one-pager — publish it to Library as an Html Property record, and hand back its URL or a share link. Use whenever a deliverable would otherwise become a Claude artifact and Library is connected, or when the user asks to put a page, report, or dashboard into Library. Covers the sandbox the page has to live within; not for editing prose records, which is the library skill.
---

# Library artifacts

A Library artifact is one whole HTML document stored in one Data record and rendered in a sandboxed frame. The record URL is the deliverable, and republishing the same Data ID keeps that URL stable.

Reach for it when the result has an audience or a life beyond this conversation: a report someone will read, a plan a team will follow, a dashboard, a prototype, a reference page. Advice the user will act on immediately, in the code in front of them, belongs in the reply instead. Use the client's own artifact tool only when the user explicitly asks for a Claude artifact, or when the page needs an artifact-only runtime (`window.claude.*`, viewer identity, stored files) — Library has none of those.

## The frame the page runs in

Library renders the value in `<iframe sandbox="allow-scripts">` with no `allow-same-origin`. The app and the server-side HTML view emit the same attribute, so every limit below applies to every page — plan around them before writing, not after a broken page comes back.

- **Opaque origin.** `localStorage`, `sessionStorage`, `document.cookie`, and IndexedDB throw on access. Nothing a viewer does survives a reload, so never build a page whose point is saving state — a poll, a checklist that must persist, a sign-up sheet. Wrap any storage access in `try/catch` so the page still renders.
- **Most of the sandbox flags are off.** `alert`, `confirm`, and `prompt` do nothing (no `allow-modals`). Form submission is blocked (no `allow-forms`) — handle input in page script instead. `window.open` and `target="_blank"` open nothing (no `allow-popups`). `<a download>` and script-driven blob saves are inert (no `allow-downloads`), so never hand the reader a file through a link; put the content in the page.
- **Links replace the artifact.** A plain link navigates the frame itself, and most sites refuse to be framed, so the reader gets a blank pane and no way back except a reload. Keep `href="#…"` anchors for in-page navigation and print external references as visible URLs.
- **No network to count on.** Requests from an opaque origin carry `Origin: null` and fail CORS at most hosts, and no CDN is guaranteed reachable. Inline every stylesheet and script, draw diagrams and charts as inline SVG, and embed images as data URIs. There is no mermaid renderer.
- **Scripts may not run at all.** A `srcdoc` document inherits the embedding page's CSP, and the desktop shell sets a strict `script-src`. Write the content into the HTML and use JavaScript only to enhance it — sorting, filtering, toggles. A page that renders nothing without JS is empty in the desktop app.
- **The frame paints white and fills its region.** Give `body` an explicit background and text color rather than borrowing one. Define the full palette on `:root`; add `prefers-color-scheme: dark` overrides only on top of it, never as the only definition of a color.

## Design the page

Sketch a plan before writing code — four to six named colors, two or three type roles, a sentence of layout — and derive every decision from it.

- **Read the request for treatment, not for effort.** A memo, a status page, or a plan wants a polished utilitarian document: real hierarchy, considered spacing, a chosen palette, no hero. A landing page, a pitch, or something the reader will keep and revisit earns an editorial treatment and one deliberate risk. Both get the same care.
- **Follow what already exists.** The user's own words first, then the organization's tokens and conventions, then your choices.
- **Ground it in the subject.** Carry at least one detail only this subject would have — its real units, its document conventions, its terms of art — as content rather than ornament. Write the real content: no lorem, no invented figures.
- **Type without a font host.** A stylesheet from `fonts.googleapis.com` is blocked by the desktop shell's CSP, so it cannot be relied on. Use a system stack, or inline the face as a `@font-face` data URI inside the size budget. Keep running text near 65 characters wide, set a type scale and stay on it, and give headings `text-wrap: balance`.
- **No libraries.** cdnjs and every other CDN are out for the same reason scripts are — React, a charting package, a syntax highlighter are all unavailable. Draw charts as inline SVG by hand: one scale places marks, ticks, and labels; leave room in the `viewBox` for the outermost labels; give every shape an explicit fill; take chart text color from the tokens.
- **Pick the neutrals.** A pure mid-grey reads as unconsidered — bias it slightly toward the accent. White and near-black are fine grounds when they are chosen rather than inherited.
- **Both themes, through tokens.** The frame carries no theme stamp, so `prefers-color-scheme` is the only signal available. Define the complete light palette on bare `:root`, redefine only tokens inside the dark media query, and style components through the tokens. A page that deliberately commits to one look may skip the dark block, but it still paints every color and its own background explicitly.
- **Let layout do the spacing.** Flex or grid with `gap` rather than per-element margins that collapse or double. `font-variant-numeric: tabular-nums` wherever digits line up in a column. Repeated things — cards in a row, label/value pairs, badges — share edges, baselines, and inner padding.
- **Not everything is a card.** Border, fill, radius, and shadow each say "separate object". Spend them on the one thing that needs lifting instead of stamping the same radius and shadow on every block, which flattens the hierarchy.
- **Structure should be true.** Numbered markers claim the content is a sequence; eyebrows and dividers claim sections. Use them only when the content actually is that.
- **Show the page at rest.** Everything meant to be read is visible on load — nothing parked at `opacity: 0` waiting for an observer, which in a shell that blocks scripts is a blank page. Size an opener to what it holds, not to `100vh`. A tool or dashboard opens in a realistic working state, with example rows plainly marked as examples.
- **Write the copy from the reader's side.** Name things as people recognize them, use active voice, and let a control say exactly what happens. Specific beats clever.
- **Build cleanly.** Close every non-void element, quote attributes, give keyboard focus a visible state, respect `prefers-reduced-motion`, and watch selector specificity so spacing rules do not cancel each other out.
- **Skip the house style of generated pages.** Warm cream with a serif display and a terracotta accent, near-black with one acid-green pop, a purple-to-blue gradient hero, Inter or Space Grotesk as the safe face, emoji as section markers, everything centered and uniformly rounded. When the user names a direction, follow it exactly — their words win, including when they ask for one of these.

There is no preview loop here: nothing renders the page back to you before it is published. Write it carefully, publish once, and fix what the user reports.

## Write the document

- Start the document with `<!doctype html>` and give it a `<title>` — a real name for the page, a short noun phrase specific to its subject, and the same one used as the record name; not a category label, and not a name with an explainer appended after a dash. Set `<html lang>` to the language the page is actually written in, not the language of the request. A value that does not begin with `<` is stored as block-editor content and never opens in the artifact frame.
- Use relative units and a single readable column (`max-width` around 60rem); the body must never scroll horizontally. Put wide tables, code blocks, and diagrams in their own `overflow-x: auto` container.
- Keep it small. `POST /mcp` accepts roughly 2 MiB per request and the Html value is capped at 3 MiB, so a photo embedded as a data URI is rejected while inline SVG and small assets are fine. Link large media by URL instead.
- Do not publish a page that impersonates a real person or organization, or that presents fabricated records as genuine.

A skeleton that satisfies the frame:

```html
<!doctype html>
<html lang="<the page's own language>">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>…</title>
    <style>
      :root { color-scheme: light; --bg: #fff; --ink: #1b1b1b; --muted: #5b6270; --line: #e5e7eb; --accent: #1d4ed8; }
      * { box-sizing: border-box; }
      body { margin: 0; background: var(--bg); color: var(--ink);
             font: 16px/1.7 system-ui, -apple-system, "Hiragino Kaku Gothic ProN", "Noto Sans JP", sans-serif; }
      main { max-width: 60rem; margin: 0 auto; padding: 2.5rem 1.25rem; }
      .scroll { overflow-x: auto; }
      table { width: 100%; border-collapse: collapse; }
      th, td { border-bottom: 1px solid var(--line); padding: 0.5rem 0.75rem; text-align: left; }
    </style>
  </head>
  <body>
    <main>…</main>
  </body>
</html>
```

## Publish it

- Send it to the organization and repository the user names. With none given, pick the organization from `list_orgs`, prefer a repository whose slug is `artifacts`, and confirm the destination once per session. Creating that repository is a write of its own — confirm first.
- `create_repo` requires `is_public`: ask whether these pages should be readable anonymously, and use `false` when only the team needs them. Pass `skip_sample_data: true`. The tool still adds a RichText Property named `content`; delete it with `delete_property`, because the client picks the page body by type and RichText and Markdown outrank Html. The repository needs exactly one body-shaped Property, of `property_type: html` — check `list_properties`, create one named `body` only when no Html Property exists, and never add a Markdown or RichText Property to an artifacts repository.
- A Data ID starts with `data_` and is lowercase: mint `data_` plus a lowercase ULID for a new page, and reuse the ID from the returned URL or the user's message for every republish of the same page.
- Call `upsert_data` with `name` as the title and one `property_data` entry `{ "property_id": <Html Property id>, "value_type": "html", "value": <full document> }`.
- Write the whole document every time. `upsert_data` replaces the value; it does not merge, version, or detect concurrent edits. Re-read with `get_data` first when the record may have been edited elsewhere, and after writing when the result needs confirming.
- Report the returned `url` and the Data ID. Members of a private repository read it after signing in; a public repository serves the same page anonymously at `/public/<org>/<repo>/<data_id>`.

## Share it outside the organization

- For a reader with no Library account, call `create_share_link` with `org`, `repo`, and `data_id`, and hand back the returned `url`. It opens that one record, read-only, without signing in.
- The token is shown once and is not recoverable, so put the URL in the same reply that mints it. A lost link can only be replaced by a new one.
- One link is one record. A second page needs a second link, and neither reaches anything else in the repository.
- Reuse the link a page already has instead of minting one per message: `list_share_links` returns them without their tokens, and `revoke_share_link` stops one working.
- A public repository is refused with a 400: `/public/<org>/<repo>/<data_id>` is already the anonymous address there.

## Before handing it back

The value starts with `<!doctype html>`, all CSS and JS are inline, the content is visible with scripts disabled, no link or button promises something the sandbox blocks (a download, a popup, saved state), and the reply carries the URL and the Data ID.
