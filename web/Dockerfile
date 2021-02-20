
FROM caddy:2-builder-alpine AS builder
ENV HTTP_PROXY=http://host.docker.internal:10809
RUN xcaddy build \
  --with github.com/caddy-dns/cloudflare


FROM node:lts-alpine as build
COPY package.json yarn.lock /app/
WORKDIR /app/
RUN yarn
COPY . /app
RUN yarn build --prod

FROM caddy:2-alpine
COPY --from=builder /usr/bin/caddy /usr/bin/caddy
COPY --from=build /app/dist/rurikawa /app
EXPOSE 80
EXPOSE 443

