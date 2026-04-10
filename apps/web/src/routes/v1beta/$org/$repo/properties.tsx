import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/v1beta/$org/$repo/properties')({
  component: PropertiesPage,
})

function PropertiesPage() {
  return <div className='container mx-auto p-4'>Properties page - TODO</div>
}
