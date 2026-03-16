export const runtime = 'edge'

import {
	ReviewListUi,
	type ReviewListUiProps,
} from '@/app/v1beta/_components/reviews-list-ui'
import { Default } from '@/app/v1beta/_components/reviews-list-ui.stories'


export default async function ReviewsPage() {
	return (
		<>
			<ReviewListUi {...(Default.args as ReviewListUiProps)} />
		</>
	)
}
