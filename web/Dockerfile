# Build the dashboard.
#
# Next.js `output: "standalone"` emits a self-contained server plus only the
# node_modules it actually uses, which keeps the runtime image small without
# hand-pruning dependencies.
FROM node:20-slim AS deps
WORKDIR /src
RUN corepack enable
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

FROM node:20-slim AS build
WORKDIR /src
RUN corepack enable
COPY --from=deps /src/node_modules ./node_modules
COPY . .
ENV NEXT_TELEMETRY_DISABLED=1 NEXT_STANDALONE=1
RUN pnpm build

FROM node:20-slim
WORKDIR /app
ENV NODE_ENV=production NEXT_TELEMETRY_DISABLED=1 PORT=3000
RUN useradd --system --uid 10001 spark

COPY --from=build /src/.next/standalone ./
COPY --from=build /src/.next/static ./.next/static
COPY --from=build /src/public ./public

USER 10001
EXPOSE 3000
CMD ["node", "server.js"]
