export const e2eAuthState = {
  cookies: [],
  origins: [
    {
      origin: 'http://127.0.0.1:5173',
      localStorage: [
        {
          name: 'library_auth',
          value: JSON.stringify({
            accessToken: 'dev:local',
            refreshToken: '',
            expiresAt: 4_102_444_800,
            userId: 'library-e2e-user',
            email: 'library-e2e@local.test',
            username: 'library-e2e',
          }),
        },
      ],
    },
  ],
}
