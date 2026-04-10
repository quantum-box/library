import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/v1beta/$org/$repo/settings')({
  component: SettingsPage,
})

function SettingsPage() {
  return <div className='container mx-auto p-4'>Settings page - TODO</div>
}
