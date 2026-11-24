FROM postgres:16.16-alpine3.24 AS patched
RUN apk upgrade --no-cache \
    && apk add --no-cache su-exec \
    && ln -sf /sbin/su-exec /usr/local/bin/gosu

# Export only the patched filesystem, without the upstream Go helper in its layers.
FROM scratch
COPY --from=patched / /
ENV LANG=en_US.utf8 PG_MAJOR=16 PGDATA=/var/lib/postgresql/data
VOLUME ["/var/lib/postgresql/data"]
EXPOSE 5432
STOPSIGNAL SIGINT
ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["postgres"]
