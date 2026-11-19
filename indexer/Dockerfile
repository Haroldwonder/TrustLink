# Stage 1: Build TypeScript
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY prisma ./prisma
RUN npx prisma generate
COPY tsconfig.json ./
COPY src ./src
COPY scripts ./scripts
RUN npm run build

# Stage 2: Production image
FROM node:20-alpine
WORKDIR /app
# Backup CronJobs invoke the scripts with Bash and need PostgreSQL client tools.
RUN apk add --no-cache bash postgresql-client
# Install only production dependencies (excludes typescript, ts-node, prisma CLI)
COPY package*.json ./
RUN npm ci --omit=dev
# Copy compiled output and generated Prisma client
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules/.prisma ./node_modules/.prisma
COPY prisma ./prisma
COPY scripts ./scripts
CMD ["sh", "-c", "npx prisma migrate deploy && node dist/index.js"]
