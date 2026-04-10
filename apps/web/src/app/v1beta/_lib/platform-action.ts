import { type GraphqlSdk, getSdkPlatform } from '@/lib/apiClient'
import { ClientError } from 'graphql-request'

export const ErrorCode = {
  TYPE_ERROR: 'TYPE_ERROR',
  INTERNAL_SERVER_ERROR: 'INTERNAL_SERVER_ERROR',
  BUSINESS_LOGIC_ERROR: 'BUSINESS_LOGIC_ERROR',
  APPLICATION_LOGIC_ERROR: 'APPLICATION_LOGIC_ERROR',
  NOT_FOUND_ERROR: 'NOT_FOUND_ERROR',
  HTTP_RESPONSE_ERROR: 'HTTP_RESPONSE_ERROR',
  UNAUTHORIZED_ERROR: 'UNAUTHORIZED_ERROR',
  PARSE_ERROR: 'PARSE_ERROR',
  ENUM_PARSE_ERROR: 'ENUM_PARSE_ERROR',
  PERMISSION_DENIED: 'PERMISSION_DENIED',
  PROVIDER_ERROR: 'PROVIDER_ERROR',
  NOT_SUPPORTED: 'NOT_SUPPORTED',
  GENERAL_ERROR: 'GENERAL_ERROR',
  UNKNOWN_ERROR: 'UNKNOWN_ERROR',
} as const

export type ErrorCodeType = (typeof ErrorCode)[keyof typeof ErrorCode]

export class PlatformActionError extends Error {
  constructor(
    message: string,
    public readonly code: ErrorCodeType,
    public readonly details?: Record<string, unknown>,
  ) {
    super(message)
    this.name = 'PlatformActionError'
  }
}

interface ErrorResponse {
  response?: {
    errors?: Array<{
      message: string
      extensions?: {
        code?: string
      }
    }>
  }
}

function classifyGraphQLError(
  extensionCode: string | undefined,
  errorMessage: string,
): ErrorCodeType {
  if (extensionCode) {
    switch (extensionCode) {
      case 'NOT_FOUND':
        return ErrorCode.NOT_FOUND_ERROR
      case 'FORBIDDEN':
        return ErrorCode.PERMISSION_DENIED
      case 'UNAUTHORIZED':
        return ErrorCode.UNAUTHORIZED_ERROR
      case 'BAD_REQUEST':
        return ErrorCode.BUSINESS_LOGIC_ERROR
      default:
        break
    }
  }
  if (errorMessage.startsWith('NotFoundError')) {
    return ErrorCode.NOT_FOUND_ERROR
  }
  if (errorMessage.startsWith('PermissionDenied')) {
    return ErrorCode.PERMISSION_DENIED
  }
  return ErrorCode.UNKNOWN_ERROR
}

/**
 * Client-side platform action. Token must be provided by the caller.
 */
export const platformAction = async <T>(
  fn: (sdk: GraphqlSdk) => Promise<T>,
  options?: {
    onError?: (error: PlatformActionError) => void | Promise<void>
    accessToken?: string
    allowAnonymous?: boolean
  },
) => {
  const sdk = options?.accessToken
    ? getSdkPlatform(options.accessToken)
    : getSdkPlatform()

  try {
    return await fn(sdk)
  } catch (err: unknown) {
    console.log('platformAction err:', err)
    let platformError: PlatformActionError

    if (err instanceof PlatformActionError) {
      platformError = err
    } else if (err instanceof ClientError) {
      const graphqlError = err.response.errors?.[0]
      if (graphqlError) {
        const errorMessage = graphqlError.message
        const extensionCode = graphqlError.extensions?.code as string | undefined
        const code = classifyGraphQLError(extensionCode, errorMessage)
        platformError = new PlatformActionError(errorMessage, code, {
          originalError: err,
          graphqlError,
        })
      } else {
        platformError = new PlatformActionError(
          'An unexpected GraphQL error occurred',
          ErrorCode.UNKNOWN_ERROR,
          { originalError: err },
        )
      }
    } else {
      const errorMessage = err instanceof Error ? err.message : String(err ?? '')
      let extractedJson: ErrorResponse | null = null
      try {
        const jsonMatch = errorMessage.match(/\{[\s\S]*\}/)
        if (jsonMatch) {
          extractedJson = JSON.parse(jsonMatch[0]) as ErrorResponse
        }
      } catch {
        // ignore
      }

      if (extractedJson?.response?.errors?.[0]) {
        const graphqlError = extractedJson.response.errors[0]
        const code =
          (graphqlError.extensions?.code as ErrorCodeType) || ErrorCode.UNKNOWN_ERROR
        platformError = new PlatformActionError(graphqlError.message, code, {
          originalError: extractedJson,
        })
      } else {
        platformError = new PlatformActionError(
          'An unexpected error occurred',
          ErrorCode.UNKNOWN_ERROR,
          { originalError: err },
        )
      }
    }

    console.error('Platform Action Error:', {
      message: platformError.message,
      code: platformError.code,
      details: platformError.details,
    })

    if (options?.onError) {
      await options.onError(platformError)
      return undefined as unknown as T
    }

    throw platformError
  }
}
