import assert from 'node:assert/strict'
import { after, test } from 'node:test'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { createFetchMock, Miniflare } from 'miniflare'

const outDir = await mkdtemp(join(tmpdir(), 'library-public-docs-test-'))
after(() => rm(outDir, { recursive: true, force: true }))
const build = spawnSync('node', ['scripts/build-workers.mjs', 'public-docs'], {
  stdio: 'inherit',
  env: { ...process.env, PUBLIC_DOCS_OUT_DIR: outDir, VITE_LIBRARY_API_BASE_URL: 'https://api.example.test' },
})
assert.equal(build.status, 0)
const scriptPath = join(outDir, '_worker.js', 'index.js')
const origin = 'https://planetlibrary.txcloud.app'
const base = '/v1beta/repos/acme/guide'
const route = '/public/acme/guide'
const profile = {
  name: 'Sample Guide',
  username: 'guide',
  description: 'A guide for readers',
  is_public: true,
}
const properties = [{ id: 'body', property_type: 'RICH_TEXT' }]
const article = {
  id: 'intro',
  name: 'Introduction',
  items: [
    {
      property_id: 'body',
      value: {
        richText: JSON.stringify([
          {
            type: 'paragraph',
            content: [{ text: 'A useful public document.' }],
            children: [],
          },
        ]),
      },
    },
  ],
}

async function scenario(t, routes = {}) {
  const calls = []
  const fetchMock = createFetchMock()
  fetchMock.disableNetConnect()
  fetchMock
    .get('https://api.example.test')
    .intercept({ path: /.*/ })
    .reply((options) => {
      calls.push(options)
      const response = routes[options.path] ?? { status: 500, data: {} }
      return {
        statusCode: response.status ?? 200,
        data: JSON.stringify(response.data),
      }
    })
    .persist()
  const mf = new Miniflare({
    modules: true,
    scriptPath,
    modulesRoot: join(outDir, '_worker.js'),
    modulesRules: [{ type: 'CompiledWasm', include: ['**/*.wasm'], fallthrough: true }],
    compatibilityDate: '2026-05-05',
    compatibilityFlags: ['nodejs_compat'],
    fetchMock,
    serviceBindings: {
      ASSETS: () =>
        new Response(
          '<!doctype html><html lang="en"><head><title>Library</title>' +
            '<meta name="description" data-app-default content="App default">' +
            '<meta property="og:title" data-app-default content="Library">' +
            '</head><body><div id="root"></div><script type="module" src="/assets/app.js"></script></body></html>',
          { headers: { 'content-type': 'text/html' } },
        ),
    },
  })
  t.after(() => mf.dispose())
  return {
    calls,
    fetch: (path = `${route}/intro`, init) =>
      mf.dispatchFetch(origin + path, init),
    mf,
  }
}

const ready = {
  [base]: { data: profile },
  [`${base}/data/intro`]: { data: article },
  [`${base}/properties`]: { data: properties },
}

test('initial HTML includes metadata and body without JS; incoming credentials never reach the API', async (t) => {
  const { fetch, calls } = await scenario(t, ready)
  const response = await fetch(`${route}/intro?utm_source=test`, {
    headers: {
      Authorization: 'Bearer test-only',
      Cookie: 'test-only=1',
      'x-operator-id': 'test-only',
    },
  })
  assert.equal(response.status, 200)
  assert.equal(response.headers.get('cache-control'), 'no-store')
  const html = await response.text()
  assert.match(html, /<title[^>]*>Introduction · Sample Guide<\/title>/)
  assert.match(html, /name="description" content="A useful public document\."/)
  assert.match(
    html,
    /rel="canonical" href="https:\/\/planetlibrary.txcloud.app\/public\/acme\/guide\/intro"/,
  )
  assert.match(html, /property="og:type" content="article"/)
  assert.match(html, /name="twitter:card" content="summary"/)
  assert.match(
    html,
    /property="og:image" content="https:\/\/planetlibrary.txcloud.app\/apple-touch-icon.png"/,
  )
  assert.match(html, /"@type":"TechArticle"/)
  // The shell's own defaults describe the app, so a document is never
  // described twice with the app's summary left standing.
  assert.doesNotMatch(html, /data-app-default/)
  assert.doesNotMatch(html, /content="App default"/)
  assert.equal(html.match(/name="description"/g).length, 1)
  assert.match(html, /<h1>Introduction<\/h1>/)
  assert.match(html, /A useful public document\./)
  assert.match(html, /src="\/assets\/app.js"/)
  assert.equal(calls.length, 3)
  for (const call of calls) {
    const headers = JSON.stringify(call.headers).toLowerCase()
    assert.doesNotMatch(headers, /test-only|authorization|cookie|x-operator-id/)
  }
})

for (const status of [401, 403, 404]) {
  test(`repository API ${status} prevents content fetches and returns an unindexed 404`, async (t) => {
    const { fetch, calls } = await scenario(t, { [base]: { status, data: {} } })
    const response = await fetch()
    assert.equal(response.status, 404)
    assert.equal(response.headers.get('x-robots-tag'), 'noindex, nofollow')
    assert.equal(calls.length, 1)
    assert.doesNotMatch(await response.text(), /Introduction|A useful public/)
  })
}

test('an authenticated-only profile cannot become a public page', async (t) => {
  const { fetch, calls } = await scenario(t, {
    [base]: { data: { ...profile, is_public: false } },
  })
  assert.equal((await fetch()).status, 404)
  assert.equal(calls.length, 1)
})

test('missing articles return 404 and API outages return retryable 503', async (t) => {
  const { fetch } = await scenario(t, {
    ...ready,
    [`${base}/data/intro`]: { status: 404, data: {} },
  })
  assert.equal((await fetch()).status, 404)
  const outage = await scenario(t, { [base]: { status: 500, data: {} } })
  const response = await outage.fetch()
  assert.equal(response.status, 503)
  assert.equal(response.headers.get('retry-after'), '60')
})

test('repository metadata and paginated sitemap expose only public URLs', async (t) => {
  const routes = {
    ...ready,
    [`${base}/data-list?page=1&page_size=100`]: {
      data: { data: [article], paginator: { total_pages: 2 } },
    },
    [`${base}/data-list?page=2&page_size=100`]: {
      data: {
        data: [{ ...article, id: 'second' }],
        paginator: { total_pages: 2 },
      },
    },
  }
  const { fetch } = await scenario(t, routes)
  const index = await (await fetch(route)).text()
  assert.match(index, /"@type":"CollectionPage"/)
  assert.match(index, /href="\/public\/acme\/guide\/intro"/)
  const sitemap = await (await fetch(`${route}/sitemap.xml`)).text()
  assert.match(sitemap, /sitemap.xml\?page=2/)
  const second = await (await fetch(`${route}/sitemap.xml?page=2`)).text()
  assert.match(second, /\/guide\/second<\/loc>/)
  assert.doesNotMatch(second, /\/guide\/intro<\/loc>/)
  assert.equal((await fetch(`${route}/sitemap.xml?page=0`)).status, 404)
})

test('preview responses stay unindexed and HEAD retains metadata headers without a body', async (t) => {
  const { mf, fetch } = await scenario(t, ready)
  const preview = await mf.dispatchFetch(
    `https://test.library-client.pages.dev${route}/intro`,
  )
  assert.equal(preview.headers.get('x-robots-tag'), 'noindex, nofollow')
  const previewHtml = await preview.text()
  assert.match(previewHtml, /name="robots" content="noindex, nofollow"/)
  // Structured data describes a page offered for indexing; a preview URL
  // is not one.
  assert.doesNotMatch(previewHtml, /application\/ld\+json/)
  const head = await fetch(undefined, { method: 'HEAD' })
  assert.equal(head.status, 200)
  assert.equal(await head.text(), '')
})

test('repository text cannot inject markup or terminate the structured data script', async (t) => {
  const name = '</script><script>alert("test")</script>'
  const { fetch } = await scenario(t, {
    ...ready,
    [`${base}/data/intro`]: { data: { ...article, name } },
  })
  const html = await (await fetch()).text()
  assert.doesNotMatch(html, /<script>alert/)
  assert.match(html, /\\u003c\/script>/)
  assert.match(html, /&lt;script&gt;alert/)
})

test('share links reach the worker and are unindexed without an API call', async (t) => {
  // The header is only worth anything if Pages routes the path here at all;
  // everything outside `include` is served straight from the asset bucket.
  const routes = JSON.parse(await readFile(join(outDir, '_routes.json'), 'utf8'))
  assert.deepEqual(routes.include, ['/public/*', '/robots.txt', '/s/*'])

  const { fetch, calls } = await scenario(t)
  const response = await fetch('/s/shr_token')
  assert.equal(response.status, 200)
  assert.equal(response.headers.get('x-robots-tag'), 'noindex, nofollow')
  // A share token is never read here: the worker adds the instruction and
  // hands back the shell, so no private document reaches a crawler.
  assert.equal(calls.length, 0)
  assert.doesNotMatch(await response.text(), /shr_token/)
})

test('robots permits only public production routes and blocks previews', async (t) => {
  const { fetch, mf, calls } = await scenario(t)
  assert.match(await (await fetch('/robots.txt')).text(), /Allow: \/public\//)
  assert.equal(
    await (
      await mf.dispatchFetch('https://preview.example.test/robots.txt')
    ).text(),
    'User-agent: *\nDisallow: /\n',
  )
  assert.equal(calls.length, 0)
})

test('malformed public paths are unindexed 404s without API calls', async (t) => {
  const { fetch, calls } = await scenario(t)
  for (const path of [
    `${route}/intro/extra`,
    '/public/acme',
    `${route}/bad%2Fid`,
    `${route}/%00`,
    `${route}/%ZZ`,
  ]) {
    const response = await fetch(path)
    assert.equal(response.status, 404, path)
    assert.equal(response.headers.get('x-robots-tag'), 'noindex, nofollow')
  }
  assert.equal(calls.length, 0)
})

test('API redirects and oversized data are rejected rather than followed or cached', async (t) => {
  const redirected = await scenario(t, { [base]: { status: 302, data: {} } })
  assert.equal((await redirected.fetch()).status, 503)
  const large = await scenario(t, {
    [base]: { data: { ...profile, description: 'x'.repeat(2 * 1024 * 1024) } },
  })
  const response = await large.fetch()
  assert.equal(response.status, 503)
  assert.equal(response.headers.get('cache-control'), 'no-store')
  assert.equal(large.calls.length, 1)
})

test('nullable repository descriptions retain the public page and empty fallback', async (t) => {
  const { fetch } = await scenario(t, { ...ready, [base]: { data: { ...profile, description: null } } })
  const response = await fetch()
  assert.equal(response.status, 200)
  assert.match(await response.text(), /A useful public document\./)
})
