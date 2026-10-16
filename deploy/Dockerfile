# The image `just deploy` uploads to Railway: the server (built for musl by scripts/deploy.sh) and the built web/.
# Nothing is compiled here.
FROM alpine:3
# The server checks the sign-in providers' certificates against the system's authorities: without them every
# call to a provider fails. The ls fails the build if the store is empty.
RUN apk add --no-cache ca-certificates && ls /etc/ssl/certs/ca-certificates.crt
WORKDIR /app
COPY cityloom_server cityloom_server
COPY web web
ENV E2E_HOST=0.0.0.0 CACHE_CONTROL=no-cache WEB_ROOT=/app/web
# Railway names the port to listen on in PORT, which the server reads.
CMD ["./cityloom_server"]
