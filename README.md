Basic plan:

- Implement enough CLI to track this project locally
- Push the issues to a remote repo, and implement sync workflows

- TBD order
  - Use the lib to implement an API that exposes the workflows over web
  - Build a local UI client (git-backed)
    - Could be UI wrapper around lib
    - Could run light version of API locally


## Getting Started:

- Build with `cargo build`
- Create a team: `mkdir -p repo/TEAM/tasks`
- Run with `just cli --help` (or `target/debug/dv`)
