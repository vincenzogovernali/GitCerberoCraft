This build contains the original game's own files, for the private use of the person who owns the game, on their own computer. It must not be shared or published.

# GitCerberoCraft

A free graphical Git client written in Rust (egui), with:

- commit graph, staging, diffs, branches, tags, stashes, undo/redo;
- **Commit and push** button in the commit panel (commit, then push the branch);
- HTTPS sign-in with the GitHub/GitLab tokens saved in Preferences > Integrations,
  used for push, pull and fetch also when the Git program is installed (the system
  credential manager is skipped for that site, so the right account is used);
- a plain explanation when GitHub/GitLab refuses a push (wrong account, missing
  `repo` scope, fine-grained token without "Contents: Read and write", SSO...);
- "Test token" buttons and `gitcerberocraft-cli check-token`;
- language files in `lang/` detected automatically;
- AI assistant by API or ACP;
- Git downloader for each operating system and version;
- a command-line tool `gitcerberocraft-cli` with an MCP server (`gitcerberocraft-cli mcp`),
  including `commit -m <msg> --push` and the MCP tool `git_commit` with `push: true`.

See CREDITS.md for the files used.
