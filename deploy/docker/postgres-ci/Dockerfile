FROM postgres:19.0-alpine AS patched
RUN apk upgrade --no-cache \
    && apk add --no-cache su-exec \
    && ln -sf /sbin/su-exec /usr/local/bin/gosu

# Export only the patched filesystem, without the upstream Go helper in its layers.
FROM scratch
COPY --from=patched / /
ENV LANG=en_US.utf8 PG_MAJOR=19 PGDATA=/var/lib/postgresql/19/docker
VOLUME ["/var/lib/postgresql"]
EXPOSE 5432
STOPSIGNAL SIGINT
ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["postgres"]
