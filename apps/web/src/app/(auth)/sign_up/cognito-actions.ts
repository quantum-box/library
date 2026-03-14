'use server'

import { generateSecretHash } from '../cognito'
import {
	CognitoIdentityProvider,
	ConfirmForgotPasswordCommand,
	ConfirmSignUpCommand,
	ForgotPasswordCommand,
	ResendConfirmationCodeCommand,
	SignUpCommand,
} from '@aws-sdk/client-cognito-identity-provider'

function getEnv(name: string) {
	const value = process.env[name]
	if (!value) {
		throw new Error(`${name} is not set`)
	}
	return value
}

function createClient() {
	return new CognitoIdentityProvider({ region: getEnv('COGNITO_REGION') })
}

function getClientInfo() {
	const clientId = getEnv('COGNITO_CLIENT_ID')
	const clientSecret = getEnv('COGNITO_CLIENT_SECRET')
	return { clientId, clientSecret }
}

export async function signUpWithCognito(input: {
	username: string
	email: string
	password: string
}) {
	const { clientId, clientSecret } = getClientInfo()
	const client = createClient()

	const command = new SignUpCommand({
		ClientId: clientId,
		Username: input.username,
		Password: input.password,
		SecretHash: generateSecretHash(input.username, clientId, clientSecret),
		UserAttributes: [{ Name: 'email', Value: input.email }],
	})

	return client.send(command)
}

export async function confirmSignUpWithCognito(input: {
	username: string
	code: string
}) {
	const { clientId, clientSecret } = getClientInfo()
	const client = createClient()

	const command = new ConfirmSignUpCommand({
		ClientId: clientId,
		Username: input.username,
		ConfirmationCode: input.code,
		SecretHash: generateSecretHash(input.username, clientId, clientSecret),
	})

	return client.send(command)
}

export async function resendConfirmationCode(input: { username: string }) {
	const { clientId, clientSecret } = getClientInfo()
	const client = createClient()

	const command = new ResendConfirmationCodeCommand({
		ClientId: clientId,
		Username: input.username,
		SecretHash: generateSecretHash(input.username, clientId, clientSecret),
	})

	return client.send(command)
}

/**
 * Request password reset - sends a 6-digit verification code to user's email
 */
export async function forgotPassword(input: { username: string }) {
	const { clientId, clientSecret } = getClientInfo()
	const client = createClient()

	const command = new ForgotPasswordCommand({
		ClientId: clientId,
		Username: input.username,
		SecretHash: generateSecretHash(input.username, clientId, clientSecret),
	})

	return client.send(command)
}

/**
 * Confirm password reset with verification code and new password
 */
export async function confirmForgotPassword(input: {
	username: string
	code: string
	newPassword: string
}) {
	const { clientId, clientSecret } = getClientInfo()
	const client = createClient()

	const command = new ConfirmForgotPasswordCommand({
		ClientId: clientId,
		Username: input.username,
		ConfirmationCode: input.code,
		Password: input.newPassword,
		SecretHash: generateSecretHash(input.username, clientId, clientSecret),
	})

	return client.send(command)
}
