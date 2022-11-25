package main

import (
	"strconv"

	"github.com/google/go-github/v42/github"
)

type eventHandler func(payload interface{}) (repo string, metadata map[string]string, err error)

var handlers = map[string]eventHandler{
	"create":      handleCreateEvent,
	"push":        handlePushEvent,
	"release":     handleReleaseEvent,
	"check_suite": handleCheckSuiteEvent,
}

func handleCreateEvent(payload interface{}) (repo string, metadata map[string]string, err error) {
	event := payload.(*github.CreateEvent)
	return *event.Repo.FullName, map[string]string{
		"GOFER_EXTENSION_GITHUB_REF":        *event.Ref,
		"GOFER_EXTENSION_GITHUB_REF_TYPE":   *event.RefType,
		"GOFER_EXTENSION_GITHUB_REPOSITORY": *event.Repo.FullName,
	}, nil
}

func handlePushEvent(payload interface{}) (repo string, metadata map[string]string, err error) {
	event := payload.(*github.PushEvent)
	return *event.Repo.FullName, map[string]string{
		"GOFER_EXTENSION_GITHUB_REF":                           *event.Ref,
		"GOFER_EXTENSION_GITHUB_REPOSITORY":                    *event.Repo.FullName,
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_ID":                *event.HeadCommit.ID,
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_AUTHOR_NAME":       *event.HeadCommit.Author.Name,
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_AUTHOR_EMAIL":      *event.HeadCommit.Author.Email,
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_AUTHOR_USERNAME":   *event.HeadCommit.Author.Login,
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_COMMITER_NAME":     *event.HeadCommit.Committer.Name,
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_COMMITER_EMAIL":    *event.HeadCommit.Committer.Email,
		"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_COMMITER_USERNAME": *event.HeadCommit.Committer.Login,
	}, nil
}

func handleReleaseEvent(payload interface{}) (repo string, metadata map[string]string, err error) {
	event := payload.(*github.ReleaseEvent)
	return *event.Repo.FullName, map[string]string{
		"GOFER_EXTENSION_GITHUB_ACTION":                   *event.Action,
		"GOFER_EXTENSION_GITHUB_REPOSITORY":               *event.Repo.FullName,
		"GOFER_EXTENSION_GITHUB_RELEASE_TAG_NAME":         *event.Release.TagName,
		"GOFER_EXTENSION_GITHUB_RELEASE_TARGET_COMMITISH": *event.Release.TargetCommitish,
		"GOFER_EXTENSION_GITHUB_RELEASE_AUTHOR_LOGIN":     *event.Release.Author.Login,
		"GOFER_EXTENSION_GITHUB_RELEASE_CREATED_AT":       event.Release.CreatedAt.String(),
		"GOFER_EXTENSION_GITHUB_RELEASE_PUBLISHED_AT":     event.Release.PublishedAt.String(),
	}, nil
}

func handleCheckSuiteEvent(payload interface{}) (repo string, metadata map[string]string, err error) {
	event := payload.(*github.CheckSuiteEvent)
	return *event.Repo.FullName, map[string]string{
		"GOFER_EXTENSION_GITHUB_ACTION":                  *event.Action,
		"GOFER_EXTENSION_GITHUB_REPOSITORY":              *event.Repo.FullName,
		"GOFER_EXTENSION_GITHUB_CHECK_SUITE_ID":          strconv.FormatInt(*event.CheckSuite.ID, 10),
		"GOFER_EXTENSION_GITHUB_CHECK_SUITE_HEAD_SHA":    *event.CheckSuite.HeadSHA,
		"GOFER_EXTENSION_GITHUB_CHECK_SUITE_HEAD_BRANCH": *event.CheckSuite.HeadBranch,
	}, nil
}
