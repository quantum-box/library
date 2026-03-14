'use server'

import { redirect } from 'next/navigation'

export async function loginAction() {
	redirect('/sign_in')
}
