# vkoxide 🦀

[![Rust CI](https://github.com/itmagelab/vkoxide/actions/workflows/ci.yml/badge.svg)](https://github.com/itmagelab/vkoxide/actions/workflows/ci.yml)

**vkoxide** is a lightweight, strictly asynchronous SDK and framework in Rust designed for building VKontakte community bots using the Bots Long Poll API.

Inspired by projects like `teloxide`, `vkoxide` prioritizes clean code, async handler ergonomics, and modular event routing. It operates natively on `tokio` and `reqwest` and parses cleanly against the current `5.199` version bounds of the VK API.

## Core Capabilities

- **Frictionless Async Handlers:** No boilerplate! Write your endpoints as normal async closures resolving straightforward `Result` combinations instead of manually pinning Futures (`Box::pin`).
- **Context & Shared State:** Custom data (such as HTTP connection pools, Redis clients, or configuration) can easily be bundled natively right into `Context<State>`. Handlers automatically receive your specific strongly-typed payload upon each message.
- **Dynamic Event Dispatching:** Create predicates or filters inside the router to cleanly separate varying message types and contexts. Stop matching huge enums on each incoming message!

## Project Navigation

For a complete working example of how to start your dispatcher, implement shared logic state and send a response along with a native bot Keyboard, check out [`examples/main.rs`](./examples/main.rs).

## VK API References

- [Bots Long Poll API — Getting Started](https://dev.vk.com/ru/api/bots/getting-started)
- [User Long Poll API](https://dev.vk.com/ru/api/user-long-poll/getting-started)
