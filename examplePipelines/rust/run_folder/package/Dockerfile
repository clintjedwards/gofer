# Runs as nobody, to show a task running as another user can still use the run folder.
FROM alpine:3.22
COPY package.sh /package.sh
USER nobody
CMD ["/bin/sh", "/package.sh"]
