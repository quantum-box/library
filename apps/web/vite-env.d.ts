/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_BACKEND_API_URL: string
  readonly VITE_PLATFORM_ID: string
  readonly VITE_COGNITO_CLIENT_ID: string
  readonly VITE_COGNITO_USER_POOL_ID: string
  readonly VITE_COGNITO_REGION: string
  readonly VITE_COGNITO_ISSUER: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
