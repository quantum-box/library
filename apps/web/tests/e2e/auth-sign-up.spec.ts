import { expect, test } from '@playwright/test'

test.describe('new account registration', () => {
	test('renders the registration form and validates visible fields in Japanese', async ({
		page,
	}) => {
		await page.goto('/sign_up')

		await expect(
			page.getByRole('heading', { name: '新規登録', exact: true }),
		).toBeVisible()
		await expect(page.getByPlaceholder('ユーザー名を入力')).toBeVisible()
		await expect(page.getByPlaceholder('メールアドレスを入力')).toBeVisible()
		await expect(page.getByPlaceholder('パスワードを入力')).toBeVisible()
		await expect(page.getByPlaceholder('パスワードを再入力')).toBeVisible()
		await expect(
			page.getByRole('button', { name: '新規登録', exact: true }),
		).toBeVisible()
		await expect(
			page.getByRole('link', { name: 'サインイン', exact: true }),
		).toHaveAttribute('href', '/sign_in')

		await page.getByPlaceholder('ユーザー名を入力').fill('ab')
		await page.getByPlaceholder('メールアドレスを入力').fill('not-email')
		await page.getByPlaceholder('パスワードを入力').fill('Password1')
		await page.getByPlaceholder('パスワードを再入力').fill('Password2')
		await page.getByRole('button', { name: '新規登録', exact: true }).click()

		await expect(
			page.getByText('ユーザー名は3文字以上で入力してください'),
		).toBeVisible()
		await expect(
			page.getByText('有効なメールアドレスを入力してください'),
		).toBeVisible()
		await expect(page.getByText('パスワードが一致しません')).toBeVisible()
		await expect(page).toHaveURL(/\/sign_up$/)
	})

	test('toggles password visibility and navigates to sign in', async ({ page }) => {
		await page.goto('/sign_up')

		const passwordInput = page.getByPlaceholder('パスワードを入力')
		await expect(passwordInput).toHaveAttribute('type', 'password')

		await page.getByRole('button', { name: '表示', exact: true }).click()
		await expect(passwordInput).toHaveAttribute('type', 'text')
		await expect(
			page.getByRole('button', { name: '非表示', exact: true }),
		).toBeVisible()

		await page.getByRole('link', { name: 'サインイン', exact: true }).click()
		await expect(page).toHaveURL(/\/sign_in$/)
		await expect(
			page.getByRole('heading', { name: 'サインイン', exact: true }),
		).toBeVisible()
	})

	test('shows the email verification page without calling Cognito', async ({
		page,
	}) => {
		await page.goto('/verify-email/otp')

		await expect(
			page.getByRole('heading', {
				name: 'メールを確認してください',
				exact: true,
			}),
		).toBeVisible()
		await expect(page.getByPlaceholder('ユーザー名を入力')).toBeVisible()
		await expect(page.getByPlaceholder('6桁のコードを入力')).toBeVisible()
		await expect(
			page.getByRole('button', { name: '認証', exact: true }),
		).toBeVisible()
		await expect(
			page.getByRole('button', { name: 'コードを再送信', exact: true }),
		).toBeVisible()
		await expect(
			page.getByRole('link', { name: 'やり直す', exact: true }),
		).toHaveAttribute('href', '/sign_up')

		await page.getByRole('button', { name: '認証', exact: true }).click()

		await expect(page.getByText('ユーザー名を入力してください')).toBeVisible()
		await expect(page.getByText('6桁の認証コードを入力してください')).toBeVisible()
		await expect(page).toHaveURL(/\/verify-email\/otp$/)
	})
})
