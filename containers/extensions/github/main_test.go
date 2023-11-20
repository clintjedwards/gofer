package main

import (
	"fmt"
	"reflect"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestParseEventFilter(t *testing.T) {
	tests := []struct {
		name        string
		input       string
		wantEvent   string
		wantActions []string
	}{
		{
			name:        "No Action Provided",
			input:       "event",
			wantEvent:   "event",
			wantActions: []string{},
		},
		{
			name:        "Single Action",
			input:       "event/action",
			wantEvent:   "event",
			wantActions: []string{"action"},
		},
		{
			name:        "Multiple Actions",
			input:       "event/action1,action2",
			wantEvent:   "event",
			wantActions: []string{"action1", "action2"},
		},
		{
			name:        "Action Casing",
			input:       "event/AcTiOn",
			wantEvent:   "event",
			wantActions: []string{"action"},
		},
		{
			name:        "Empty Actions",
			input:       "event/",
			wantEvent:   "event",
			wantActions: []string{},
		},
		{
			name:        "Action With Trailing Comma",
			input:       "event/action1,",
			wantEvent:   "event",
			wantActions: []string{"action1"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			gotEvent, gotActions := parseEventFilter(tt.input)
			if gotEvent != tt.wantEvent {
				t.Errorf("Expected event %s, got %s", tt.wantEvent, gotEvent)
			}
			if !reflect.DeepEqual(gotActions, tt.wantActions) {
				fmt.Printf("WANT: %T\n", tt.wantActions)
				fmt.Printf("GOT: %T\n", gotActions)
				t.Errorf("Expected actions %v, got %v", tt.wantActions, gotActions)
			}
		})
	}
}

func TestMatchSubscriptions(t *testing.T) {
	tests := []struct {
		name              string
		extensionSetup    extension
		event             string
		repo              string
		action            string
		wantSubscriptions []pipelineSubscription
	}{
		{
			name: "No Subscriptions",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{},
			},
			event:             "push",
			repo:              "repo1",
			action:            "create",
			wantSubscriptions: []pipelineSubscription{},
		},
		{
			name: "Match By Event And Repo",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {
							{actions: map[string]struct{}{"create": {}}},
						},
					},
				},
			},
			event:  "push",
			repo:   "repo1",
			action: "",
			wantSubscriptions: []pipelineSubscription{
				{actions: map[string]struct{}{"create": {}}},
			},
		},
		{
			name: "Action Case Insensitive",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {
							{actions: map[string]struct{}{"create": {}}},
						},
					},
				},
			},
			event:  "push",
			repo:   "repo1",
			action: "CREATE",
			wantSubscriptions: []pipelineSubscription{
				{actions: map[string]struct{}{"create": {}}},
			},
		},
		{
			name: "Match Multiple Actions",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {
							{actions: map[string]struct{}{"create": {}, "delete": {}}},
						},
					},
				},
			},
			event:  "push",
			repo:   "repo1",
			action: "delete",
			wantSubscriptions: []pipelineSubscription{
				{actions: map[string]struct{}{"create": {}, "delete": {}}},
			},
		},
		{
			name: "Event and Repo Match, Action Does Not",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {
							{actions: map[string]struct{}{"create": {}}},
						},
					},
				},
			},
			event:             "push",
			repo:              "repo1",
			action:            "update",
			wantSubscriptions: []pipelineSubscription{},
		},
		{
			name: "Event Does Not Exist",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {{actions: map[string]struct{}{"create": {}}}},
					},
				},
			},
			event:             "pull_request",
			repo:              "repo1",
			action:            "create",
			wantSubscriptions: []pipelineSubscription{},
		},
		{
			name: "Multiple Repos, One Match",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {{actions: map[string]struct{}{"create": {}}}},
						"repo2": {{actions: map[string]struct{}{"delete": {}}}},
					},
				},
			},
			event:  "push",
			repo:   "repo1",
			action: "create",
			wantSubscriptions: []pipelineSubscription{
				{actions: map[string]struct{}{"create": {}}},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			gotSubscriptions := tt.extensionSetup.matchSubscriptions(tt.event, tt.repo, tt.action)

			// Using cmp for deep comparisons
			if diff := cmp.Diff(tt.wantSubscriptions, gotSubscriptions, cmp.AllowUnexported(pipelineSubscription{})); diff != "" {
				t.Errorf("mismatch (-want +got):\n%s", diff)
			}
		})
	}
}

func TestCastPayload() {
	test := `
 {
	"ref": "refs/heads/master",
	"before": "826bfebe2e8133c7378628944108f92f462e09f8",
	"after": "a2b06ee91f7bd49f2abeb5e0913534776d985b00",
	"repository": {
	  "id": 229465078,
	  "node_id": "MDEwOlJlcG9zaXRvcnkyMjk0NjUwNzg=",
	  "name": "experimental",
	  "full_name": "clintjedwards/experimental",
	  "private": false,
	  "owner": {
		"name": "clintjedwards",
		"email": "clintjedwards@users.noreply.github.com",
		"login": "clintjedwards",
		"id": 3713842,
		"node_id": "MDQ6VXNlcjM3MTM4NDI=",
		"avatar_url": "https://avatars.githubusercontent.com/u/3713842?v=4",
		"gravatar_id": "",
		"url": "https://api.github.com/users/clintjedwards",
		"html_url": "https://github.com/clintjedwards",
		"followers_url": "https://api.github.com/users/clintjedwards/followers",
		"following_url": "https://api.github.com/users/clintjedwards/following{/other_user}",
		"gists_url": "https://api.github.com/users/clintjedwards/gists{/gist_id}",
		"starred_url": "https://api.github.com/users/clintjedwards/starred{/owner}{/repo}",
		"subscriptions_url": "https://api.github.com/users/clintjedwards/subscriptions",
		"organizations_url": "https://api.github.com/users/clintjedwards/orgs",
		"repos_url": "https://api.github.com/users/clintjedwards/repos",
		"events_url": "https://api.github.com/users/clintjedwards/events{/privacy}",
		"received_events_url": "https://api.github.com/users/clintjedwards/received_events",
		"type": "User",
		"site_admin": false
	  },
	  "html_url": "https://github.com/clintjedwards/experimental",
	  "description": "Test repository for testing various software automation and processes",
	  "fork": false,
	  "url": "https://github.com/clintjedwards/experimental",
	  "forks_url": "https://api.github.com/repos/clintjedwards/experimental/forks",
	  "keys_url": "https://api.github.com/repos/clintjedwards/experimental/keys{/key_id}",
	  "collaborators_url": "https://api.github.com/repos/clintjedwards/experimental/collaborators{/collaborator}",
	  "teams_url": "https://api.github.com/repos/clintjedwards/experimental/teams",
	  "hooks_url": "https://api.github.com/repos/clintjedwards/experimental/hooks",
	  "issue_events_url": "https://api.github.com/repos/clintjedwards/experimental/issues/events{/number}",
	  "events_url": "https://api.github.com/repos/clintjedwards/experimental/events",
	  "assignees_url": "https://api.github.com/repos/clintjedwards/experimental/assignees{/user}",
	  "branches_url": "https://api.github.com/repos/clintjedwards/experimental/branches{/branch}",
	  "tags_url": "https://api.github.com/repos/clintjedwards/experimental/tags",
	  "blobs_url": "https://api.github.com/repos/clintjedwards/experimental/git/blobs{/sha}",
	  "git_tags_url": "https://api.github.com/repos/clintjedwards/experimental/git/tags{/sha}",
	  "git_refs_url": "https://api.github.com/repos/clintjedwards/experimental/git/refs{/sha}",
	  "trees_url": "https://api.github.com/repos/clintjedwards/experimental/git/trees{/sha}",
	  "statuses_url": "https://api.github.com/repos/clintjedwards/experimental/statuses/{sha}",
	  "languages_url": "https://api.github.com/repos/clintjedwards/experimental/languages",
	  "stargazers_url": "https://api.github.com/repos/clintjedwards/experimental/stargazers",
	  "contributors_url": "https://api.github.com/repos/clintjedwards/experimental/contributors",
	  "subscribers_url": "https://api.github.com/repos/clintjedwards/experimental/subscribers",
	  "subscription_url": "https://api.github.com/repos/clintjedwards/experimental/subscription",
	  "commits_url": "https://api.github.com/repos/clintjedwards/experimental/commits{/sha}",
	  "git_commits_url": "https://api.github.com/repos/clintjedwards/experimental/git/commits{/sha}",
	  "comments_url": "https://api.github.com/repos/clintjedwards/experimental/comments{/number}",
	  "issue_comment_url": "https://api.github.com/repos/clintjedwards/experimental/issues/comments{/number}",
	  "contents_url": "https://api.github.com/repos/clintjedwards/experimental/contents/{+path}",
	  "compare_url": "https://api.github.com/repos/clintjedwards/experimental/compare/{base}...{head}",
	  "merges_url": "https://api.github.com/repos/clintjedwards/experimental/merges",
	  "archive_url": "https://api.github.com/repos/clintjedwards/experimental/{archive_format}{/ref}",
	  "downloads_url": "https://api.github.com/repos/clintjedwards/experimental/downloads",
	  "issues_url": "https://api.github.com/repos/clintjedwards/experimental/issues{/number}",
	  "pulls_url": "https://api.github.com/repos/clintjedwards/experimental/pulls{/number}",
	  "milestones_url": "https://api.github.com/repos/clintjedwards/experimental/milestones{/number}",
	  "notifications_url": "https://api.github.com/repos/clintjedwards/experimental/notifications{?since,all,participating}",
	  "labels_url": "https://api.github.com/repos/clintjedwards/experimental/labels{/name}",
	  "releases_url": "https://api.github.com/repos/clintjedwards/experimental/releases{/id}",
	  "deployments_url": "https://api.github.com/repos/clintjedwards/experimental/deployments",
	  "created_at": 1576951645,
	  "updated_at": "2022-01-22T06:51:05Z",
	  "pushed_at": 1706603198,
	  "git_url": "git://github.com/clintjedwards/experimental.git",
	  "ssh_url": "git@github.com:clintjedwards/experimental.git",
	  "clone_url": "https://github.com/clintjedwards/experimental.git",
	  "svn_url": "https://github.com/clintjedwards/experimental",
	  "homepage": "",
	  "size": 42,
	  "stargazers_count": 0,
	  "watchers_count": 0,
	  "language": "Go",
	  "has_issues": true,
	  "has_projects": true,
	  "has_downloads": true,
	  "has_wiki": true,
	  "has_pages": false,
	  "has_discussions": false,
	  "forks_count": 0,
	  "mirror_url": null,
	  "archived": false,
	  "disabled": false,
	  "open_issues_count": 3,
	  "license": null,
	  "allow_forking": true,
	  "is_template": false,
	  "web_commit_signoff_required": false,
	  "topics": [

	  ],
	  "visibility": "public",
	  "forks": 0,
	  "open_issues": 3,
	  "watchers": 0,
	  "default_branch": "master",
	  "stargazers": 0,
	  "master_branch": "master"
	},
	"pusher": {
	  "name": "clintjedwards",
	  "email": "clintjedwards@users.noreply.github.com"
	},
	"sender": {
	  "login": "clintjedwards",
	  "id": 3713842,
	  "node_id": "MDQ6VXNlcjM3MTM4NDI=",
	  "avatar_url": "https://avatars.githubusercontent.com/u/3713842?v=4",
	  "gravatar_id": "",
	  "url": "https://api.github.com/users/clintjedwards",
	  "html_url": "https://github.com/clintjedwards",
	  "followers_url": "https://api.github.com/users/clintjedwards/followers",
	  "following_url": "https://api.github.com/users/clintjedwards/following{/other_user}",
	  "gists_url": "https://api.github.com/users/clintjedwards/gists{/gist_id}",
	  "starred_url": "https://api.github.com/users/clintjedwards/starred{/owner}{/repo}",
	  "subscriptions_url": "https://api.github.com/users/clintjedwards/subscriptions",
	  "organizations_url": "https://api.github.com/users/clintjedwards/orgs",
	  "repos_url": "https://api.github.com/users/clintjedwards/repos",
	  "events_url": "https://api.github.com/users/clintjedwards/events{/privacy}",
	  "received_events_url": "https://api.github.com/users/clintjedwards/received_events",
	  "type": "User",
	  "site_admin": false
	},
	"installation": {
	  "id": 45179900,
	  "node_id": "MDIzOkludGVncmF0aW9uSW5zdGFsbGF0aW9uNDUxNzk5MDA="
	},
	"created": false,
	"deleted": false,
	"forced": false,
	"base_ref": null,
	"compare": "https://github.com/clintjedwards/experimental/compare/826bfebe2e81...a2b06ee91f7b",
	"commits": [
	  {
		"id": "a2b06ee91f7bd49f2abeb5e0913534776d985b00",
		"tree_id": "3a8c6bb17441c94e09266e3662bb54e6ebddd1e3",
		"distinct": true,
		"message": "tes2",
		"timestamp": "2024-01-30T03:26:35-05:00",
		"url": "https://github.com/clintjedwards/experimental/commit/a2b06ee91f7bd49f2abeb5e0913534776d985b00",
		"author": {
		  "name": "Clint J Edwards",
		  "email": "clint.j.edwards@gmail.com",
		  "username": "clintjedwards"
		},
		"committer": {
		  "name": "Clint J Edwards",
		  "email": "clint.j.edwards@gmail.com",
		  "username": "clintjedwards"
		},
		"added": [
		  "test"
		],
		"removed": [

		],
		"modified": [

		]
	  }
	],
	"head_commit": {
	  "id": "a2b06ee91f7bd49f2abeb5e0913534776d985b00",
	  "tree_id": "3a8c6bb17441c94e09266e3662bb54e6ebddd1e3",
	  "distinct": true,
	  "message": "tes2",
	  "timestamp": "2024-01-30T03:26:35-05:00",
	  "url": "https://github.com/clintjedwards/experimental/commit/a2b06ee91f7bd49f2abeb5e0913534776d985b00",
	  "author": {
		"name": "Clint J Edwards",
		"email": "clint.j.edwards@gmail.com",
		"username": "clintjedwards"
	  },
	  "committer": {
		"name": "Clint J Edwards",
		"email": "clint.j.edwards@gmail.com",
		"username": "clintjedwards"
	  },
	  "added": [
		"test"
	  ],
	  "removed": [

	  ],
	  "modified": [

	  ]
	}
  }`

	_ = test
}

// func TestMatchSubscriptions(t *testing.T) {
// 	extension := extension{
// 		subscriptions: map[string]map[string][]pipelineSubscription{},
// 	}

// 	_, err := extension.Subscribe(context.Background(), &proto.ExtensionSubscribeRequest{
// 		PipelineExtensionLabel: "test_extension",
// 		NamespaceId:            "test_namespace",
// 		PipelineId:             "test_pipeline",
// 		Config: map[string]string{
// 			"events":     "push,create",
// 			"repository": "clintjedwards/experimental",
// 		},
// 	})
// 	if err != nil {
// 		t.Fatal(err)
// 	}

// 	subs1 := extension.matchSubscriptions("create", "clintjedwards/experimental")
// 	subs2 := extension.matchSubscriptions("pull_request", "clintjedwards/experimental")

// 	result1 := []pipelineSubscription{
// 		{
// 			eventFilter:    "create",
// 			repository:     "clintjedwards/experimental",
// 			extensionLabel: "test_extension",
// 			namespace:      "test_namespace",
// 			pipeline:       "test_pipeline",
// 		},
// 	}

// 	result2 := []pipelineSubscription{}

// 	if diff := cmp.Diff(result1, subs1, cmp.AllowUnexported(pipelineSubscription{})); diff != "" {
// 		t.Errorf("mismatch (-want +got):\n%s", diff)
// 	}

// 	if diff := cmp.Diff(result2, subs2, cmp.AllowUnexported(pipelineSubscription{})); diff != "" {
// 		t.Errorf("mismatch (-want +got):\n%s", diff)
// 	}
// }
