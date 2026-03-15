import { notFound } from 'next/navigation'
import { ErrorCode, platformAction } from '../v1beta/_lib/platform-action'


export default async function ErrTest() {
	await platformAction(
		async sdk => {
			return await sdk.ErrTest()
		},
		{
			onError: error => {
				if (error.code === ErrorCode.NOT_FOUND_ERROR) {
					notFound()
				}
				throw error
			},
			redirectOnError: false,
		},
	)

	return <div>Success</div>
}
