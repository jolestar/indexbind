import { openWebIndex } from '../../../dist/cloudflare.js';

const bundleBaseUrl = 'https://mdorigin-search.invalid/search/index.bundle/';

export default {
  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === '/api/search') {
      try {
        const query = url.searchParams.get('q') ?? 'rust guide';
        const mode = url.searchParams.get('mode') ?? 'virtual';
        const index =
          mode === 'direct'
            ? await openWebIndex(new URL('/search/index.bundle/', url.origin))
            : await openVirtualBundleIndex(url.origin);
        const hits = await index.search(query);
        return Response.json({
          query,
          mode,
          topHit: hits[0]?.relativePath ?? null,
          score: hits[0]?.score ?? null,
          count: hits.length,
        });
      } catch (error) {
        return Response.json(
          {
            error: error instanceof Error ? (error.stack ?? error.message) : String(error),
          },
          { status: 500 },
        );
      }
    }

    if (url.pathname === '/healthz') {
      return new Response('ok');
    }

    return fetch(request);
  },
};

async function openVirtualBundleIndex(assetOrigin: string) {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    const requestUrl =
      typeof input === 'string'
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    if (requestUrl.startsWith(bundleBaseUrl)) {
      const relativePath = requestUrl.slice(bundleBaseUrl.length);
      const assetUrl = new URL(`/search/index.bundle/${relativePath}`, assetOrigin);
      return originalFetch(assetUrl, init);
    }

    return originalFetch(input as RequestInfo, init);
  };

  try {
    return await openWebIndex(new URL(bundleBaseUrl));
  } finally {
    globalThis.fetch = originalFetch;
  }
}
