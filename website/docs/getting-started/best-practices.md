---
id: best-practices
title: Best Practices
sidebar_position: 3
---

# Best Practices

In order to schedule workloads on Gofer your code will need to be wrapped in a docker container. This is a short workflow blurb about how to create containers to best work with Gofer.

## 1) Write your code to be idempotent.

Write your code in whatever language you want, but it's a good idea to make it idempotent. Gofer does not guarantee single container runs (but even if it did that doesn't prevent mistakes from users).

## 2) Follow [12-factor best practices.](https://12factor.net)

[Configuration](https://12factor.net/config) is the important one. Gofer manages information into containers by environment variables so your code will need to take any input or configuration it needs from environment variables.

## 3) Keep things simple.

You could, in theory, create a super complicated graph of containers that run off each other. But the main theme of Gofer is simplicity. Make sure you're thinking through the benefits of managing something in separate containers vs just running a monolith container. There are good reasons for both; always err on the side of clarity and ease of understanding.

## 4) Keep your containers lean.

Because of the potentially distributed nature of Gofer, the larger the containers you run, the greater potential lag time between the start of execution for your container. This is because there is no guarantee that your container will end up on a machine that already has the image. Downloading large images takes a lot of time and a lot of disk space.
