Basic plan:

- Implement enough CLI to track this project locally
- Push the issues to a remote repo, and implement sync workflows

- TBD order
  - Use the lib to implement an API that exposes the workflows over web
  - Build a local UI client (git-backed)
    - Could be UI wrapper around lib
    - Could run light version of API locally


## Getting Started:
Install `libsodium-dev`, pijul, rust, and just

- Create a team: `mkdir -p repo/TEAM/tasks`
- Initialize the team's repo: `cd repo && pijul init`
- Configure pijul identity with the 'default' username: `pijul identity new`
- Create a database file: `just db-setup`
- Build with `cargo build`
- Run with `just cli --help` (or `target/debug/dv`)
