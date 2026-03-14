'use client'

import { redirect } from 'next/navigation'

export default function VerifyEmailRedirectPage() {
	// Always send users to the OTP UI
	redirect('/verify-email/otp')
}
