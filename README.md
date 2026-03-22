# 🧠 MyKnowledge (`mk`)

A blazing-fast, frictionless, local-first knowledge base and note-taking tool built entirely in Rust. 

Designed for developers who want to capture ideas, code snippets, and terminal commands without ever leaving the command line or breaking their flow.

---

## ✨ Features

- **⚡ Lightning Fast:** Compiled in Rust for instant startup and execution.
- **🗂️ Context-Aware:** Organize notes into "Buckets" (Contexts). The CLI remembers where you are working so you don't have to type out tags every time.
- **✍️ Interactive Workflow:** Frictionless `mk add` command with multi-line Markdown support.
- **🔍 Precision Search:** Globally search across all your notes, or strictly within tags and content.
- **🛡️ Data Ownership:** 100% local. Your data lives entirely on your hard drive in plain text formats.
- **🏗️ Hexagonal Architecture:** Built using strict Domain-Driven Design (DDD) principles for maximum maintainability.

---

## 🚀 Installation

Ensure you have [Rust and Cargo](https://rustup.rs/) installed on your system.

Clone the repository and install it globally:

```bash
git clone https://github.com/Dushyanthyadav/MyKnowledge.git
cd myknowledge
cargo install --path .