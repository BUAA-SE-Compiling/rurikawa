FROM node:lts-alpine as build
COPY package.json yarn.lock /app/
WORKDIR /app/
RUN yarn
COPY . /app
RUN yarn build --prod

FROM caddy:2-alpine
COPY --from=build /app/dist/rurikawa /app
EXPOSE 80
EXPOSE 443

