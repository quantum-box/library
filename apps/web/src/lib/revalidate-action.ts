'use server'
import { revalidatePath, revalidateTag } from 'next/cache'
import 'server-only'

export async function revalidatePathAction(path: string) {
	'use server'
	revalidatePath(path)
}

export async function revalidateTagAction(tag: string) {
	'use server'
	revalidateTag(tag)
}
