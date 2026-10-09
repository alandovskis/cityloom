# The image `just deploy` uploads to Railway: the static server the e2e tests use, over the built web/.
FROM node:24-alpine
WORKDIR /app
COPY e2e/serve.mjs e2e/serve.mjs
COPY web web
ENV E2E_HOST=0.0.0.0 CACHE_CONTROL=no-cache
# Railway names the port to listen on in PORT.
CMD ["sh", "-c", "E2E_PORT=$PORT exec node e2e/serve.mjs"]
