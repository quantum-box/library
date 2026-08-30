import { zodResolver } from '@hookform/resolvers/zod'
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { Loader2 } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { useForm } from 'react-hook-form'
import { z } from 'zod'
import { useAuth } from '@/auth'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { restClient } from '@/lib/apiClient'
import { useToastWithError } from '@/lib/error-toast'
import { executeGraphQL, graphql } from '@/lib/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'

type AccessibleTenant = {
  tenantId: string
  name: string
  username: string
  /**
   * Members of the tenant, or `null` when the API could not count them:
   * listing a tenant's users is itself permissioned. A missing count must
   * not be rendered as `0`, which reads as an empty tenant.
   */
  staffCount: number | null
  hasLibraryOrg: boolean
  canImportToLibrary: boolean
}

const AccessibleTenantsQuery = graphql(`
  query AccessibleTenants {
    accessibleTenants {
      tenantId
      name
      username
      staffCount
      hasLibraryOrg
      canImportToLibrary
    }
  }
`)

const SeedLibraryTenantMutation = graphql(`
  mutation SeedLibraryTenantFromNewOrg($tenantId: String!) {
    seedLibraryTenant(tenantId: $tenantId) {
      organization {
        username
      }
    }
  }
`)

export const Route = createFileRoute('/v1beta/organization/new')({
  component: NewOrganizationPage,
})

function NewOrganizationPage() {
  const navigate = useNavigate()
  const { session, isLoading: isAuthLoading } = useAuth()
  const { t } = useTranslation()
  const { toast, errorToast } = useToastWithError()

  const [accessibleTenants, setAccessibleTenants] = useState<AccessibleTenant[]>(
    [],
  )
  const [tenantsLoading, setTenantsLoading] = useState(true)
  const [selectedTenantId, setSelectedTenantId] = useState<string>('')
  const [importing, setImporting] = useState(false)

  const formSchema = z.object({
    name: z.string().min(1, t.v1beta.newOrg.validation.minLength.replace('{min}', '1')),
    username: z
      .string()
      .min(3, t.v1beta.newOrg.validation.minLength.replace('{min}', '3'))
      .max(40, t.v1beta.newOrg.validation.maxLength.replace('{max}', '40'))
      .regex(/^[a-zA-Z0-9_-]+$/, {
        message: t.v1beta.newOrg.validation.invalidFormat,
      }),
  })

  type NewOrgFormValues = z.infer<typeof formSchema>

  const form = useForm<NewOrgFormValues>({
    resolver: zodResolver(formSchema),
    reValidateMode: 'onChange',
    defaultValues: {
      name: '',
      username: '',
    },
  })

  const selectedTenant = useMemo(
    () => accessibleTenants.find((tenant) => tenant.tenantId === selectedTenantId),
    [accessibleTenants, selectedTenantId],
  )

  useEffect(() => {
    const loadTenants = async () => {
      if (!session?.user?.accessToken) {
        setTenantsLoading(false)
        return
      }

      try {
        const result = await executeGraphQL<{
          accessibleTenants: AccessibleTenant[]
        }>(AccessibleTenantsQuery, undefined, {
          accessToken: session.user.accessToken,
        })
        const tenants = [...result.accessibleTenants].sort((a, b) => {
          const byName = a.name.localeCompare(b.name)
          if (byName !== 0) return byName
          const byUsername = a.username.localeCompare(b.username)
          if (byUsername !== 0) return byUsername
          return a.tenantId.localeCompare(b.tenantId)
        })
        setAccessibleTenants(tenants)
        const firstImportable = tenants.find(
          (tenant) => !tenant.hasLibraryOrg,
        )
        if (firstImportable) {
          setSelectedTenantId(firstImportable.tenantId)
        } else if (tenants.length > 0) {
          setSelectedTenantId(tenants[0].tenantId)
        }
      } catch (error) {
        errorToast(error)
      } finally {
        setTenantsLoading(false)
      }
    }

    void loadTenants()
  }, [session?.user?.accessToken, errorToast])

  const importTenant = async () => {
    if (!session?.user?.accessToken || !selectedTenant) {
      return
    }
    if (selectedTenant.hasLibraryOrg) {
      navigate({ to: `/v1beta/${selectedTenant.username}` })
      return
    }
    if (!selectedTenant.canImportToLibrary) {
      toast({
        variant: 'destructive',
        title: t.v1beta.newOrg.importToLibrary,
        description: t.v1beta.newOrg.insufficientTenantRole,
      })
      return
    }

    setImporting(true)
    try {
      const result = await executeGraphQL<{
        seedLibraryTenant: { organization: { username: string } }
      }>(
        SeedLibraryTenantMutation,
        { tenantId: selectedTenant.tenantId },
        { accessToken: session.user.accessToken },
      )

      toast({
        title: t.v1beta.newOrg.importToLibrary,
        description: result.seedLibraryTenant.organization.username,
      })

      navigate({
        to: `/v1beta/${result.seedLibraryTenant.organization.username}`,
      })
    } catch (error) {
      errorToast(error)
    } finally {
      setImporting(false)
    }
  }

  const onSubmit = async (values: NewOrgFormValues) => {
    if (!session?.user) {
      toast({
        variant: 'destructive',
        title: t.v1beta.newOrg.signInRequired,
        description: t.v1beta.newOrg.signInDescription,
      })
      return
    }

    try {
      const created = await restClient(session.user.accessToken).v1beta.orgs.$post({
        body: {
          name: values.name,
          username: values.username,
        },
      })

      toast({
        title: t.v1beta.newOrg.createOrganization,
        description: created.username,
      })

      navigate({ to: `/v1beta/${created.username}` })
    } catch (error) {
      errorToast(error)
    }
  }

  if (isAuthLoading) {
    return (
      <div className='container mx-auto flex items-center justify-center p-4 min-h-[60vh]'>
        <div className='h-8 w-8 animate-spin rounded-full border-b-2 border-primary' />
      </div>
    )
  }

  if (!session?.user) {
    return (
      <div className='container mx-auto p-4'>
        <Card className='mx-auto mt-10 w-full max-w-xl'>
          <CardHeader>
            <CardTitle>{t.v1beta.newOrg.title}</CardTitle>
            <CardDescription>{t.v1beta.newOrg.description}</CardDescription>
          </CardHeader>
          <CardContent className='space-y-4'>
            <p className='text-sm text-muted-foreground'>
              {t.v1beta.newOrg.signInDescription}
            </p>
            <Button asChild>
              <Link to='/sign_in'>{t.v1beta.newOrg.signInRequired}</Link>
            </Button>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className='container mx-auto p-4'>
      <Card className='mx-auto mt-6 w-full max-w-xl'>
        <CardHeader>
          <CardTitle>{t.v1beta.newOrg.title}</CardTitle>
          <CardDescription>{t.v1beta.newOrg.description}</CardDescription>
        </CardHeader>
        <CardContent className='space-y-8'>
          <section className='space-y-4'>
            <div className='space-y-1'>
              <Label htmlFor='tachyon-tenant-select'>
                {t.v1beta.newOrg.tachyonTenant}
              </Label>
              <p className='text-sm text-muted-foreground'>
                {t.v1beta.newOrg.tachyonTenantDescription}
              </p>
            </div>

            {tenantsLoading ? (
              <div className='flex items-center gap-2 text-sm text-muted-foreground'>
                <Loader2 className='h-4 w-4 animate-spin' />
                <span>{t.v1beta.newOrg.loadingTenants}</span>
              </div>
            ) : accessibleTenants.length === 0 ? (
              <p className='text-sm text-muted-foreground'>
                {t.v1beta.newOrg.noTenants}
              </p>
            ) : (
              <div className='flex flex-col gap-3 sm:flex-row sm:items-end'>
                <div className='flex-1 space-y-2'>
                  <Select
                    value={selectedTenantId}
                    onValueChange={setSelectedTenantId}
                  >
                    <SelectTrigger id='tachyon-tenant-select'>
                      <SelectValue
                        placeholder={t.v1beta.newOrg.tachyonTenantPlaceholder}
                      />
                    </SelectTrigger>
                    <SelectContent>
                      {accessibleTenants.map((tenant) => (
                        <SelectItem key={tenant.tenantId} value={tenant.tenantId}>
                          {tenant.name} (@{tenant.username})
                          {tenant.hasLibraryOrg
                            ? ` - ${t.v1beta.newOrg.alreadyInLibrary}`
                            : ''}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  {selectedTenant && !selectedTenant.hasLibraryOrg ? (
                    <p className='text-xs text-muted-foreground'>
                      {!selectedTenant.canImportToLibrary
                        ? t.v1beta.newOrg.insufficientTenantRole
                        : selectedTenant.staffCount === null
                          ? t.v1beta.newOrg.staffMembersUnknown
                          : t.v1beta.newOrg.staffMembers.replace(
                              '{count}',
                              String(selectedTenant.staffCount),
                            )}
                    </p>
                  ) : null}
                </div>
                <Button
                  type='button'
                  className='w-full sm:w-auto'
                  disabled={
                    !selectedTenant ||
                    importing ||
                    (!selectedTenant.hasLibraryOrg &&
                      !selectedTenant.canImportToLibrary)
                  }
                  onClick={() => void importTenant()}
                >
                  {importing ? (
                    <Loader2 className='h-4 w-4 animate-spin' />
                  ) : selectedTenant?.hasLibraryOrg ? (
                    t.v1beta.newOrg.openInLibrary
                  ) : (
                    t.v1beta.newOrg.importToLibrary
                  )}
                </Button>
              </div>
            )}
          </section>

          <div className='relative'>
            <div className='absolute inset-0 flex items-center'>
              <span className='w-full border-t' />
            </div>
            <div className='relative flex justify-center text-xs uppercase'>
              <span className='bg-card px-2 text-muted-foreground'>
                {t.v1beta.newOrg.orCreateManually}
              </span>
            </div>
          </div>

          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)} className='space-y-6'>
              <FormField
                control={form.control}
                name='name'
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>{t.v1beta.newOrg.name}</FormLabel>
                    <FormControl>
                      <Input
                        {...field}
                        placeholder={t.v1beta.newOrg.namePlaceholder}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name='username'
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Username</FormLabel>
                    <FormControl>
                      <Input
                        {...field}
                        placeholder={t.v1beta.newOrg.userNamePlaceholder}
                      />
                    </FormControl>
                    <p className='text-xs text-muted-foreground'>
                      {t.v1beta.newOrg.userNameHint}
                    </p>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <Button type='submit' disabled={form.formState.isSubmitting}>
                {form.formState.isSubmitting
                  ? t.v1beta.newOrg.creating
                  : t.v1beta.newOrg.createOrganization}
              </Button>
            </form>
          </Form>

          <div className='rounded-md border border-muted p-4'>
            <p className='text-sm font-medium'>{t.v1beta.newOrg.tips}</p>
            <ul className='mt-2 list-inside list-disc text-sm text-muted-foreground'>
              <li>{t.v1beta.newOrg.tipsList.lowercaseOnly}</li>
              <li>{t.v1beta.newOrg.tipsList.keepShort}</li>
            </ul>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
