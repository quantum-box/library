import { useAuth } from '@/auth'
import { Button } from '@/components/ui/button'

export function SignOutButton() {
	const { signOut } = useAuth()
	const handleSignOut = () => {
		signOut()
	}
	return <Button onClick={handleSignOut}>Sign out</Button>
}
