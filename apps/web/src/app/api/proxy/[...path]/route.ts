export const runtime = 'edge'

import { baseURL } from '@/lib/apiClient'
import { NextRequest, NextResponse } from 'next/server'


/**
 * Proxy API requests to the backend (library-api)
 *
 * This allows client-side components to access the backend API through
 * the Next.js server, avoiding CORS issues and simplifying URL management
 * between Docker and host environments.
 *
 * Client → /api/proxy/v1/graphql → Next.js → library-api:50053/v1/graphql
 */
export async function GET(
	request: NextRequest,
	{ params }: { params: Promise<{ path: string[] }> },
) {
	return proxyRequest(request, params)
}

export async function POST(
	request: NextRequest,
	{ params }: { params: Promise<{ path: string[] }> },
) {
	return proxyRequest(request, params)
}

export async function PUT(
	request: NextRequest,
	{ params }: { params: Promise<{ path: string[] }> },
) {
	return proxyRequest(request, params)
}

export async function DELETE(
	request: NextRequest,
	{ params }: { params: Promise<{ path: string[] }> },
) {
	return proxyRequest(request, params)
}

export async function PATCH(
	request: NextRequest,
	{ params }: { params: Promise<{ path: string[] }> },
) {
	return proxyRequest(request, params)
}

async function proxyRequest(
	request: NextRequest,
	params: Promise<{ path: string[] }>,
) {
	const { path } = await params
	const targetPath = `/${path.join('/')}`
	const targetUrl = `${baseURL}${targetPath}`

	// Forward query parameters
	const url = new URL(request.url)
	const queryString = url.search

	const fullUrl = `${targetUrl}${queryString}`

	// Build headers, forwarding relevant ones
	const headers = new Headers()
	const forwardHeaders = [
		'content-type',
		'authorization',
		'x-operator-id',
		'x-platform-id',
		'x-user-id',
		'x-authenticated-userid',
		'x-idempotency-key',
		'accept',
	]

	for (const header of forwardHeaders) {
		const value = request.headers.get(header)
		if (value) {
			headers.set(header, value)
		}
	}

	try {
		const body =
			request.method !== 'GET' && request.method !== 'HEAD'
				? await request.text()
				: undefined

		const response = await fetch(fullUrl, {
			method: request.method,
			headers,
			body,
		})

		// Forward response headers
		const responseHeaders = new Headers()
		const copyHeaders = ['content-type', 'cache-control', 'etag']
		for (const header of copyHeaders) {
			const value = response.headers.get(header)
			if (value) {
				responseHeaders.set(header, value)
			}
		}

		const responseBody = await response.text()

		return new NextResponse(responseBody, {
			status: response.status,
			headers: responseHeaders,
		})
	} catch (error) {
		console.error('[Proxy] Request failed:', error)
		return NextResponse.json(
			{ error: 'Backend request failed', details: String(error) },
			{ status: 502 },
		)
	}
}
