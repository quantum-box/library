export const runtime = 'edge'

import { platformAction } from '../v1beta/_lib/platform-action'
import { handleNotFoundOrThrow } from '../v1beta/_lib/platform-error-handler'


export default async function ErrTest() {
	await platformAction(
		async sdk => {
			return await sdk.ErrTest()
		},
		{
			onError: handleNotFoundOrThrow,
			redirectOnError: false,
		},
	)

	return <div>Success</div>
}
