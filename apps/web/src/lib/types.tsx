export type Result<T> = {
	status: 'success' | 'error'
	message?: string
	meta?: T
}
