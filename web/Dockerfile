# Build a Caddy server with custom extensions
FROM caddy:2-builder-alpine AS builder
RUN xcaddy build \
  --with github.com/caddy-dns/cloudflare

# Build webpage
FROM node:lts-alpine as build
COPY package.json yarn.lock /app/
WORKDIR /app/
RUN yarn
COPY . /app
RUN yarn build --prod

# Move into running environment
FROM caddy:2-alpine
COPY --from=builder /usr/bin/caddy /usr/bin/caddy
COPY --from=build /app/dist/rurikawa /app
EXPOSE 80
EXPOSE 443

