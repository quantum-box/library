import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/v1beta/$org/$repo/data')({
  component: DataPage,
})

function DataPage() {
  return <div className='container mx-auto p-4'>Data page - TODO</div>
}
