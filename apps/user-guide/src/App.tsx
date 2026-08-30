import { Layout } from '@/components/layout'
import { ApiKeys } from '@/pages/api-keys'
import { GettingStarted } from '@/pages/getting-started'
import { GraphQL } from '@/pages/graphql'
import { Rest } from '@/pages/rest'
import { Navigate, Route, Routes } from 'react-router-dom'

export function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<GettingStarted />} />
        <Route path='api-keys' element={<ApiKeys />} />
        <Route path='rest' element={<Rest />} />
        <Route path='graphql' element={<GraphQL />} />
        <Route path='*' element={<Navigate to='/' replace />} />
      </Route>
    </Routes>
  )
}
