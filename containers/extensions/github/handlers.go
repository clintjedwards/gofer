package main

import (
	"fmt"

	"github.com/google/go-github/v58/github"
)

type eventHandler func(payload interface{}) (repo, action string, metadata map[string]string, err error)

var handlers = map[string]eventHandler{
	"pull_request":            handlePullRequestEvent,
	"pull_request_with_check": handlePullRequestEvent,
	"push":                    handlePushEvent,
	"release":                 handleReleaseEvent,
}

func safeDeref(ptr *string) string {
	if ptr != nil {
		return *ptr
	}
	return ""
}

func handlePullRequestEvent(payload interface{}) (repo, action string, metadata map[string]string, err error) {
	event, ok := payload.(*github.PullRequestEvent)
	if !ok {
		return "", "", nil, fmt.Errorf("could not cast payload to *github.PullRequestEvent")
	}

	action = safeDeref(event.Action)

	return *event.Repo.FullName, action, map[string]string{
		"GOFER_EXTENSION_GITHUB_EVENT":                       "pull_request",
		"GOFER_EXTENSION_GITHUB_ACTION":                      action,
		"GOFER_EXTENSION_GITHUB_PULLREQUEST_HEAD_REF":        safeDeref(event.PullRequest.Head.Ref),
		"GOFER_EXTENSION_GITHUB_PULLREQUEST_BRANCH":          safeDeref(event.PullRequest.Head.Ref),
		"GOFER_EXTENSION_GITHUB_REPOSITORY":                  safeDeref(event.Repo.FullName),
		"GOFER_EXTENSION_GITHUB_PULLREQUEST_HEAD_SHA":        safeDeref(event.PullRequest.Head.SHA),
		"GOFER_EXTENSION_GITHUB_PULLREQUEST_AUTHOR_USERNAME": safeDeref(event.PullRequest.User.Login),
		"GOFER_EXTENSION_GITHUB_PULLREQUEST_AUTHOR_EMAIL":    safeDeref(event.PullRequest.User.Email),
		"GOFER_EXTENSION_GITHUB_PULLREQUEST_AUTHOR_NAME":     safeDeref(event.PullRequest.User.Name),
	}, nil
}

func handlePushEvent(payload interface{}) (repo, action string, metadata map[string]string, err error) {
	event, ok := payload.(*github.PushEvent)
	if !ok {
		return "", "", nil, fmt.Errorf("could not cast payload to *github.PushEvent")
	}

	action = safeDeref(event.Action)

	return *event.Repo.FullName, action, map[string]string{
		"GOFER_EXTENSION_GITHUB_EVENT":                          "push",
		"GOFER_EXTENSION_GITHUB_ACTION":                         action,
		"GOFER_EXTENSION_GITHUB_REF":                            safeDeref(event.Ref),
		"GOFER_EXTENSION_GITHUB_REPOSITORY":                     safeDeref(event.Repo.FullName),
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_ID":                 safeDeref(event.HeadCommit.ID),
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_AUTHOR_NAME":        safeDeref(event.HeadCommit.Author.Name),
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_AUTHOR_EMAIL":       safeDeref(event.HeadCommit.Author.Email),
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_AUTHOR_USERNAME":    safeDeref(event.HeadCommit.Author.Login),
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_COMMITTER_NAME":     safeDeref(event.HeadCommit.Committer.Name),
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_COMMITTER_EMAIL":    safeDeref(event.HeadCommit.Committer.Email),
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_COMMITTER_USERNAME": safeDeref(event.HeadCommit.Committer.Login),
	}, nil
}

func handleReleaseEvent(payload interface{}) (repo, action string, metadata map[string]string, err error) {
	event, ok := payload.(*github.ReleaseEvent)
	if !ok {
		return "", "", nil, fmt.Errorf("could not cast payload to *github.ReleaseEvent")
	}

	action = safeDeref(event.Action)

	return *event.Repo.FullName, action, map[string]string{
		"GOFER_EXTENSION_GITHUB_EVENT":                    "release",
		"GOFER_EXTENSION_GITHUB_ACTION":                   action,
		"GOFER_EXTENSION_GITHUB_REPOSITORY":               safeDeref(event.Repo.FullName),
		"GOFER_EXTENSION_GITHUB_RELEASE_TAG_NAME":         safeDeref(event.Release.TagName),
		"GOFER_EXTENSION_GITHUB_RELEASE_TARGET_COMMITISH": safeDeref(event.Release.TargetCommitish),
		"GOFER_EXTENSION_GITHUB_RELEASE_AUTHOR_LOGIN":     safeDeref(event.Release.Author.Login),
		"GOFER_EXTENSION_GITHUB_RELEASE_CREATED_AT":       event.Release.CreatedAt.String(),
		"GOFER_EXTENSION_GITHUB_RELEASE_PUBLISHED_AT":     event.Release.PublishedAt.String(),
	}, nil
}
