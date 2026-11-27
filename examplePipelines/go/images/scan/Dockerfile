# Anything copy and run can't do can be a Dockerfile. Gofer builds it like `docker build ./scan` would.
FROM alpine:3.22
COPY check.sh /check.sh
CMD ["/bin/sh", "/check.sh"]
