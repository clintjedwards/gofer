---
id: overview
title: Overview
sidebar_position: 1
---

# Notifiers

Notifiers are Gofer's way of allowing finished pipelines to report their status. This is useful for a myriad of reasons:

1. Reporting successful CI run back to a particular PR.
2. Alerting a failed pipeline run or pattern of failed pipeline runs to an operator.
3. Logging something particular within a pipeline.
4. ETC...

## Supported Notifiers

| name                | image                                                       | included | description                                     |
| ------------------- | ----------------------------------------------------------- | -------- | ----------------------------------------------- | --- |
| [log](log/overview) | ghcr.io/clintjedwards/gofer-containers/notifiers/log:latest | yes      | Log prints the status of the last run to stdout |     |

## How to add new Notifiers?

Just like [tasks](../pipeline-configuration/task/task-stanza), notifiers are simply docker containers! Making them easily testable and portable. To create a new notifier you simply use the included [Gofer SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).
