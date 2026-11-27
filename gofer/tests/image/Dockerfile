# The image every hurl test registers its tasks with. `make run-hurl-tests` builds it and unpacks it into fixtures/.
#
# Each task says what it does in its SCRIPT variable, so this one image can stand in for all of them. Without one, a
# task does nothing and succeeds.
#
# It has to stay a single layer, since the tests upload it as exactly three files: the layer, the config and the
# manifest. Alpine is one layer, and CMD doesn't add one.
FROM alpine:3.22
CMD ["/bin/sh", "-c", "eval \"$SCRIPT\""]
