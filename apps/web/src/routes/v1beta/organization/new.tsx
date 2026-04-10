import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/v1beta/organization/new')({
  component: NewOrganizationPage,
})

function NewOrganizationPage() {
  return <div className='container mx-auto p-4'>New Organization - TODO</div>
}
