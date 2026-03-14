import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuPortal,
	DropdownMenuSeparator,
	DropdownMenuShortcut,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
	ChevronDown,
	Cloud,
	CreditCard,
	Eye,
	FileText,
	Github,
	Keyboard,
	LifeBuoy,
	LogOut,
	Mail,
	MessageSquare,
	Plus,
	PlusCircle,
	Settings,
	Star,
	User,
	UserPlus,
	Users,
} from 'lucide-react'
import { useSession } from 'next-auth/react'
import Link from 'next/link'

export function HeaderAccountAvatar({
	copyCount = 0,
	starCount = 0,
}: {
	copyCount?: number
	starCount?: number
}) {
	const { data: session } = useSession()
	// const session = await auth()
	if (!session) return <div />
	return (
		<>
			<Button variant='outline' size='sm'>
				<Eye className='w-4 h-4 mr-2' />
				Subscribe
				<ChevronDown className='w-4 h-4 ml-2' />
			</Button>
			<Button variant='outline' size='sm'>
				<FileText className='w-4 h-4 mr-2' />
				Copy
				{copyCount > 0 && (
					<Badge variant='secondary' className='ml-2'>
						{copyCount}
					</Badge>
				)}
			</Button>
			<Button variant='outline' size='sm'>
				<Star className='w-4 h-4 mr-2' />
				Star
				{starCount > 0 && (
					<Badge variant='secondary' className='ml-2'>
						{starCount}
					</Badge>
				)}
			</Button>
			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Avatar>
						<AvatarImage
							src='https://github.com/shadcn.png'
							alt='@shadcn'
							sizes='24px'
						/>
						<AvatarFallback>CN</AvatarFallback>
					</Avatar>
				</DropdownMenuTrigger>
				<DropdownMenuContent className='w-56'>
					<DropdownMenuLabel>
						My Account
						<br />
						{session.user?.email}
					</DropdownMenuLabel>
					<DropdownMenuSeparator />
					<DropdownMenuGroup>
						<DropdownMenuItem>
							<User className='mr-2 h-4 w-4' />
							<span>Profile</span>
							<DropdownMenuShortcut>⇧⌘P</DropdownMenuShortcut>
						</DropdownMenuItem>
						<DropdownMenuItem>
							<CreditCard className='mr-2 h-4 w-4' />
							<span>Billing</span>
							<DropdownMenuShortcut>⌘B</DropdownMenuShortcut>
						</DropdownMenuItem>
						<DropdownMenuItem>
							<Settings className='mr-2 h-4 w-4' />
							<span>Settings</span>
							<DropdownMenuShortcut>⌘S</DropdownMenuShortcut>
						</DropdownMenuItem>
						<DropdownMenuItem>
							<Keyboard className='mr-2 h-4 w-4' />
							<span>Keyboard shortcuts</span>
							<DropdownMenuShortcut>⌘K</DropdownMenuShortcut>
						</DropdownMenuItem>
					</DropdownMenuGroup>
					<DropdownMenuSeparator />
					<DropdownMenuGroup>
						<DropdownMenuItem>
							<Users className='mr-2 h-4 w-4' />
							<span>Team</span>
						</DropdownMenuItem>
						<DropdownMenuSub>
							<DropdownMenuSubTrigger>
								<UserPlus className='mr-2 h-4 w-4' />
								<span>Invite users</span>
							</DropdownMenuSubTrigger>
							<DropdownMenuPortal>
								<DropdownMenuSubContent>
									<DropdownMenuItem>
										<Mail className='mr-2 h-4 w-4' />
										<span>Email</span>
									</DropdownMenuItem>
									<DropdownMenuItem>
										<MessageSquare className='mr-2 h-4 w-4' />
										<span>Message</span>
									</DropdownMenuItem>
									<DropdownMenuSeparator />
									<DropdownMenuItem>
										<PlusCircle className='mr-2 h-4 w-4' />
										<span>More...</span>
									</DropdownMenuItem>
								</DropdownMenuSubContent>
							</DropdownMenuPortal>
						</DropdownMenuSub>
						<DropdownMenuItem>
							<Plus className='mr-2 h-4 w-4' />
							<span>New Team</span>
							<DropdownMenuShortcut>⌘+T</DropdownMenuShortcut>
						</DropdownMenuItem>
					</DropdownMenuGroup>
					<DropdownMenuSeparator />
					<DropdownMenuItem>
						<Github className='mr-2 h-4 w-4' />
						<span>GitHub</span>
					</DropdownMenuItem>
					<DropdownMenuItem>
						<LifeBuoy className='mr-2 h-4 w-4' />
						<span>Support</span>
					</DropdownMenuItem>
					<DropdownMenuItem disabled>
						<Cloud className='mr-2 h-4 w-4' />
						<span>API</span>
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuItem asChild>
						<Link href='/sign_out'>
							<LogOut className='mr-2 h-4 w-4' />
							<span>Log out</span>
							<DropdownMenuShortcut>⇧⌘Q</DropdownMenuShortcut>
						</Link>
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>
		</>
	)
}
