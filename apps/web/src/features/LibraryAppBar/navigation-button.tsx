export function NavigationButton({
	children,
	...props
}: { children: React.ReactNode } & React.HTMLAttributes<HTMLButtonElement>) {
	return (
		<button
			className='w-[30px] h-[30px] p-[5px] bg-white rounded border border-black border-opacity-10 justify-center items-center gap-2.5 inline-flex hover:bg-slate-100'
			type='button'
			{...props}
		>
			{children}
		</button>
	)
}
