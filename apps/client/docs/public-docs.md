# Public repository documentation reader

Tracking: PLT-4350

The existing `/public/:organization/:repository` route presents a read-only documentation index. `/public/:organization/:repository/:dataId` presents an article with repository navigation, a heading outline, and previous/next links. Existing public URLs keep working.

Repository section tabs expose "Open public docs" (new tab) and "Copy URL" only after an anonymous profile read confirms the repository is public. Clipboard failures are reported without claiming success.

Content comes from the Library API: the repository name and description form the index; Data names form navigation and cards. The existing `getBodyProperty` selection prefers RichText, then legacy Markdown and Html. `RecordBodyEditor` renders that property with `editable={false}`. No properties or copies of Data are created.

All profile, list, and detail requests explicitly use `anonymous: true`. A profile must be public before list or detail reads begin. The repository gate is keyed by organization/repository so navigation cannot reuse another repository's accepted profile. Direct links to unavailable articles display an explicit error state.

Navigation preserves the API's ordering. Search covers loaded article names, with a visible load-more control and scope hint when more pages exist. Pagination appends and deduplicates Data IDs. Previous/next links cover the loaded navigation only. The heading outline observes rendered body headings, including the editor's asynchronous document initialization.

This release changes the public reader UI; it does not add SSR, custom domains, article-level publishing, category/order properties, or full-repository search. Public repos remain wholly public. The standalone design prototype lives under `prototypes/public-docs` and is not shipped in the client.

Validation:

- `npm run type-check`
- `npm test -- src/components/public/PublicDocsView.test.tsx src/components/public/PublicDataView.test.tsx src/components/public/PublicRepositoryView.test.tsx src/i18n/catalogs.test.ts`
- `npx eslint src/components/public/PublicDocsView.tsx src/components/public/PublicDocsView.test.tsx src/components/public/PublicShell.tsx`
- `npm run build`
- Local fixture API: anonymous article rendering, generated outline, `contenteditable=false`, 390px navigation and article switching. This is fixture verification, not production content or deployment verification.
