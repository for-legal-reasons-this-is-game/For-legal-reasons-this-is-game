# ---- build ----
FROM node:24.21-alpine as build

RUN apk update && apk add curl

WORKDIR /app

COPY src/package.json src/package-lock.json src/tsconfig.json src/tsconfig.node.json src/tsconfig.app.json src/vite.config.ts ./

COPY src/index.html ./
COPY src/src/ ./src

RUN npm install -D vite
RUN npm ci

RUN npm run build

# ---- run ----
FROM nginx:1.30.5-alpine

CMD ["sh"]