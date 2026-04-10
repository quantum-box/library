import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/v1beta/$org/$repo/api')({
  component: ApiPage,
})

function ApiPage() {
  return <div className='container mx-auto p-4'>API page - TODO</div>
}
