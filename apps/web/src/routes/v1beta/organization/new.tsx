import { zodResolver } from '@hookform/resolvers/zod'
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
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
import { useToastWithError } from '@/lib/error-toast'
import { restClient } from '@/lib/apiClient'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { useForm } from 'react-hook-form'
import { z } from 'zod'

export const Route = createFileRoute('/v1beta/organization/new')({
  component: NewOrganizationPage,
})

function NewOrganizationPage() {
  const navigate = useNavigate()
  const { session, isLoading: isAuthLoading } = useAuth()
  const { t } = useTranslation()
  const { toast, errorToast } = useToastWithError()

  const formSchema = z.object({
    name: z.string().min(1, 'Organization name is required.'),
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

  const onSubmit = async (values: NewOrgFormValues) => {
    if (!session?.user) {
      toast({
        variant: 'destructive',
        title: 'Sign in required',
        description: 'Please sign in to create a new organization.',
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
              Sign in to create a new organization.
            </p>
            <Button asChild>
              <Link to='/sign_in'>Sign in</Link>
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
        <CardContent>
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)} className='space-y-6'>
              <FormField
                control={form.control}
                name='name'
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Organization name</FormLabel>
                    <FormControl>
                      <Input
                        {...field}
                        placeholder={t.v1beta.newOrg.userNamePlaceholder}
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
                  ? 'Creating...'
                  : t.v1beta.newOrg.createOrganization}
              </Button>
            </form>
          </Form>

          <div className='mt-6 rounded-md border border-muted p-4'>
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
