FROM alpine:latest AS git-clone

# Install Git
RUN apk add --no-cache git

# Define build arguments
ARG GIT_URL
ARG REPO_NAME

# Clone the repository
WORKDIR /workspace
RUN git clone "$GIT_URL" "$REPO_NAME"

# Define build arguments
ARG GITHUB_AUTH

# Generate the Docker auth file
RUN --mount=type=secret,id=github_token \
    mkdir -p /kaniko/.docker && \
    echo "{\"auths\":{\"ghcr.io\":{\"auth\":\"${GITHUB_AUTH}\"}}}" > /kaniko/.docker/config.json

FROM gcr.io/kaniko-project/executor:latest AS kaniko

# Copy the cloned repository
COPY --from=git-clone /workspace /workspace
# Copy the Kaniko config
COPY --from=git-clone /kaniko/.docker/config.json /kaniko/.docker/config.json

# Define working directory
ARG REPO_NAME
WORKDIR /workspace/${REPO_NAME}

# Entrypoint for Kaniko
ENTRYPOINT ["/kaniko/executor"]
CMD ["--dockerfile=/context/Dockerfile_inner", "--context=.", "--destination=ghcr.io/clintjedwards/gofer-repo:latest"]
