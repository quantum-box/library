import { platformAction } from '@/app/v1beta/_lib/platform-action'
import type { CreateApiKeyMutation, GetApiKeysQuery } from '@/gen/graphql'

/**
 * API keys are issued per organization, so every call names the
 * organization rather than the repository the caller happens to be
 * looking at.
 */
export async function fetchApiKeys(
	orgUsername: string,
	accessToken?: string,
): Promise<GetApiKeysQuery> {
	return platformAction(sdk => sdk.getApiKeys({ orgUsername }), {
		accessToken,
	})
}

export async function createApiKey(
	orgUsername: string,
	name: string,
	accessToken?: string,
): Promise<CreateApiKeyMutation> {
	return platformAction(
		sdk =>
			sdk.createAPIKey({
				input: { organizationUsername: orgUsername, name },
			}),
		{ accessToken },
	)
}

export async function revokeApiKey(
	orgUsername: string,
	apiKeyId: string,
	accessToken?: string,
): Promise<void> {
	await platformAction(
		sdk =>
			sdk.revokeAPIKey({
				input: { organizationUsername: orgUsername, apiKeyId },
			}),
		{ accessToken },
	)
}
