FROM mysql:9.7.4.1 AS patched
RUN microdnf -y upgrade \
    && microdnf -y remove mysql-shell \
    && microdnf -y install util-linux \
    && microdnf clean all
COPY --chmod=0755 gosu /usr/local/bin/gosu

# Export the patched files without retaining the replaced programs in lower layers.
FROM scratch
COPY --from=patched / /
ENV PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
VOLUME ["/var/lib/mysql"]
EXPOSE 3306 33060
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["mysqld"]
