---
id: intro
title: Introduction
sidebar_position: 0
---

# Introduction

Welcome to the Gofer documentation! This documentation is a reference for all available features and options of Gofer.

<p align="center">
<iframe width="560" height="315" src="https://www.youtube.com/embed/wqDNYcT0XOo" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>
</p>

## What is Gofer?

Gofer is a simple, opinionated, cloud-native, container-focused, continuous thing do-er.

You deploy it as a single static binary service, pass it declarative configurations, and watch as it automatically handles periodic scheduling of your automation workloads. (Run them on whatever your
choice is of container scheduler: Nomad, K8s, Local Docker)

It's meant to run short term jobs such as: code linters, build tools, tests, port-scanners, ETL tooling and anything else you can package into a Docker container and run as a result of something happening.

## Gofer's Philosophy

_Things should be **easy and fast**. For if they are not, people will look for an alternate solution._

Gofer focuses on the usage of common docker containers to run workloads that don't belong as long-running applications. The ability to run containers easily, is powerful tool for users who need to run various short-term workloads and don't want to care about the idiosyncrasies of the tooling that they run on top of.

## How do I use Gofer? What's a common workflow?

1. Create a docker container with the workload/code you want to run.
2. Create a configuration file (kept with your workload code) in which you tell Gofer what containers to run and when they should be run.
3. Gofer takes care of the rest!

## What problem is Gofer attempting to solve?

The current landscape for running short-term jobs is heavily splintered and could do with some centralization and sanity.

### 1) Tooling in this space is often CI/CD focused and treats [gitops](https://about.gitlab.com/topics/gitops/) as a core tenet.

This is actually a good thing in most cases and something that most small companies should embrace. The guarantees and structure of gitops is useful for building and testing software.

Eventually as your workload grows though, you'll start to notice that tying your short-term job runner to a particular repository leaves a few holes in the proper management of those jobs.

**Let's take a look at an example:**

Imagine you've built a tool that uses static analysis to examine PRs for potential issues[^1]. The philosophy of gitops would have you store your tool's job settings in that same repository.

[Here is an example of buildkite's approach](https://buildkite.com/docs/pipelines/defining-steps#customizing-the-pipeline-upload-path) where your job definition is uploaded on every run via the buildkite config file at that certain commit.

This model of joining your job to the repository and the commit that repository is operating on works well until you have to fix something.

Suddenly you have to fix a bug in your job and it's a breaking change with other parts of the system. Not terribly uncommon.

#### Here is how it would work in the realm of long-running jobs traditionally:

1. You fix the bug
2. You push your code
3. You create a new release
4. You update to the new version.

Done! The users of your app see the breakage fix instantly.

#### Here is how it would work in an workload tied to gitops

1. You fix the bug
2. You push your code
3. All users who are working in the most recent commit are happy.
4. All previous users who are working in an old commit are terribly unhappy as they get weird breakage messages from their trusted static analysis tooling.

This is due to the lack of operator led deployment mechanism for gitops related tooling. If you have to make a breaking change it's either each user rebases or they're broken until further notice.

#### This leads to a poor user experience for the users who rely on that job and a poor operator experience for those who maintain it.

In theory, this should happen infrequently, but when it does its a headache. You can try to version everything and always maintain backwards compatibility for these small workloads, but not only is sometimes that not possible, but then you run into some more questions:

- How long do you support the old versions?
- How do you tell when there are no more users with a branch older than your new version?
- How do you inform users that the error they've received just needs a simple rebase?
- How do you potentially test new versions of your job to make sure they work?
- How do you implement reliability strategies like canarying or A/B testing?

#### _How does Gofer help?_

Instead of tying itself to gitops wholly, Gofer leaves it as an option for the job implementer. Each pipeline exists independent of a particular repository, while providing the job operator the ability to use triggers to still implement gitops related features.

### 2) Tooling in this space can lack testability.

Ever set up a CI/CD pipeline for your team and end up with a string of commits simply testing or fixing bugs in your assumptions of the system? This is usually due to not understanding how the system works, what values it will produce, or testing being difficult.

These are issues because most CI/CD systems make it hard to test locally. In order to support a wide array of job types most of them run custom agents which in turn run the jobs you want.

This can be bad, since it's usually non-trivial to understand exactly what these agents will do once they handle your workload. Dealing with these agents can also be an operational burden. Operators are generally unfamiliar with these custom agents and it doesn't play to the strengths of an ops team that is already focused on other complex systems.

#### _How does Gofer help?_

Gofer plays to the strengths that both operators and users already have. Instead of implementing a custom agent, instead Gofer runs all containers via an already configured cluster that operators can just point it to. This makes it so the people running your workloads are already familiar with what is happening once the container starts running and users who are already familiar with containers can implement the same testing they already know.

All Gofer does is run the same container you have know locally and pass it the environment variables you expect. Easy!

### 3) Tooling in this space can lack simplicity.

Some user experience issues I've run into using other common CI/CD tooling:

- 100 line bash script (filled with sed and awk) to configure the agent's environment before my workload was loaded onto it.
- Debugging docker in docker issues.
- Reading the metric shit ton of documentation just to get a project started.
- Trying to understand a groovy script nested so deep it got into one of the layers of hell.
- Dealing with the security issues of a way too permissive plugin system.
- Agents giving vague and indecipherable errors to why my job failed.

#### _How does Gofer help?_

Gofer aims to use tooling that users are already are familiar with and get out of the way. Running containers should be _easy_. Forgoing things like custom agents and being opinionated in how workloads should be run, allows users to understand the system immediately and be productive quickly.

Just run containers, it really is that easy.

## Why should you not use Gofer?

### 1) The code you run is not idempotent.

Gofer does not guarantee a single run of a container. Even though it does a good job in best effort, a perfect storm of operator error, trigger errors, or sudden shutdowns could cause multiple runs of the same container.

### 2) The code you run does not follow cloud native best practices.

The easiest primer on cloud native best practices is the [12-factor guide](https://12factor.net/), specifically the [configuration section](https://12factor.net/config). Gofer provides tooling for container to operate following these guidelines with the most important being that your code will need to take configuration via environment variables.

### 3) The scheduling you need is precise.

Gofer makes a best effort to start jobs on their defined timeline, but it is at the mercy of many parts of the system (scheduling lag, image download time, competition with other pipelines). If you need precise down to the second or minute runs of code Gofer does not guarantee such a thing.

Gofer works better when jobs are expected to run +1 to +5 mins of their scheduled event/time.

## Why not use <insert favorite tool here\>?

| Tool                                  | Category                         | Why not?                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Jenkins                               | General thing-doer               | Supports generally anything you might want to do ever, but because of this it can be operationally hard to manage, usually has massive security issues and isn't by default opinionated enough to provide users a good interface into how they should be managing their workloads. <br /><br />This leads to pandemonium with user experience as users string together seven levels of indentation deep groovy to do what should be operationally simple things.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Buildkite/CircleCI/Github actions/etc | Gitops cloud builders            | Gitops focused cloud build tooling is great for most situations and probably what most people/companies should start out using. The issue is that running your workloads can be hard to test since these tools use custom agents to manage those jobs. This causes local testing to be difficult as the custom agents generally work very differently locally. Many times users will fight with yaml and make commits just to test that their job does what they need due to their being no way to determine that beforehand. <br /><br /> If you want control of where your code runs you also are either completely out of luck or have to spin up custom agents that you probably don't understand operationally.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ArgoCD                                | Kubernetes focused CI/CD tooling | In the right direction with its focus on running containers on already established container orchstrators, but Argo is tied to gitops making it hard to test locally. It also focuses heavily on Kubernetes as its chosen orchestrator, which causes it to be a bit inflexible for other uses cases.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ConcourseCI                           | Container focused thing do-er    | Concourse is great and where much of this inspiration for this project comes from. It sports a sleek CLI, great UI, and cloud-native primitives that makes sense. The drawback of concourse is that it uses a custom way of managing docker containers that can be hard to reason about. This makes testing locally difficult and running in production means that your short-lived containers exist on a platform that the rest of your company is not used to running containers on.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Airflow                               | ETL systems                      | I haven't worked with large scale data systems enough to know deeply about how ETL systems came to be, but (maybe naively) they seem to fit into the same paradigm of "run _x_ thing every time _y_ happens". Airflow was particularly rough to operate in the early days of its release with security and UX around DAG runs/management being nearly non-existent. As an added bonus the scheduler regularly crashed from poorly written user workloads making it a reliability nightmare. <br /><br /> Additionally, Airflow's models of combining the execution logic of your DAGs with your code led to issues of testing and iterating locally. <br /><br /> Instead of having tooling specifically for data workloads, instead it might be easier for both data teams and ops teams to work in the model of distributed cron as Gofer does. Write your stream processing using dedicated tooling/libraries like [Benthos](https://www.benthos.dev/) (or in whatever language you're most familiar with), wrap it in a Docker container, and use Gofer to manage which containers should run when, where, and how often. This gives you easy testing, separation of responsibilities, and no python decorator spam around your logic. |

[^1]: _cough cough_ https://github.com/clintjedwards/hclvet
