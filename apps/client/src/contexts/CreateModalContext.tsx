/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext } from 'react'

export const CreateModalContext = createContext<{
  open: boolean
  setOpen: (open: boolean) => void
}>({ open: false, setOpen: () => {} })

export function useCreateModal() {
  return useContext(CreateModalContext)
}
