# ---- build ----
FROM node:24.21-alpine AS build

WORKDIR /app

COPY package.json package-lock.json ./

RUN npm ci

COPY index.html tsconfig.json tsconfig.node.json tsconfig.app.json vite.config.ts ./ 

COPY src/ ./src/

RUN npm run build

# ---- run ----
FROM nginx:1.30.5-alpine

RUN rm -rf /etc/nginx/conf.d/default.conf

COPY server.conf /etc/nginx/conf.d/

COPY --from=build /app/dist /dist/

CMD ["nginx", "-g", "daemon off;"]
