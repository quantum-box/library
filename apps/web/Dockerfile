FROM node:20-alpine AS builder
RUN apk add --no-cache libc6-compat
RUN apk update
WORKDIR /app

# Install dependencies
COPY package.json yarn.lock ./
COPY apps/web/package.json ./apps/web/
RUN yarn install --frozen-lockfile --network-timeout 600000

# Copy source
COPY . .

# Build args for public env vars
ARG NEXT_PUBLIC_BACKEND_API_URL
ENV NEXT_PUBLIC_BACKEND_API_URL=$NEXT_PUBLIC_BACKEND_API_URL

RUN yarn workspace library build

FROM node:20-alpine AS runner
WORKDIR /app

RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs
USER nextjs

COPY --from=builder /app/apps/web/next.config.js .
COPY --from=builder /app/apps/web/package.json .

# Leverage output traces to reduce image size
COPY --from=builder --chown=nextjs:nodejs /app/apps/web/.next/standalone .
COPY --from=builder --chown=nextjs:nodejs /app/apps/web/.next/static ./apps/web/.next/static
COPY --from=builder --chown=nextjs:nodejs /app/apps/web/public ./apps/web/public

CMD node ./apps/web/server.js
