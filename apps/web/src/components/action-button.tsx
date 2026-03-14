import { Button } from './ui/button'

export function ActionButton({
	action,
	children,
	...props
}: {
	action: React.FormHTMLAttributes<HTMLFormElement>['action']
} & React.ComponentPropsWithRef<typeof Button>) {
	return (
		<form action={action}>
			<Button {...props}>{children}</Button>
		</form>
	)
}
