# TODO

## Infrastructure

- [ ] Handle `# rubocop:disable` / `# rubocop:enable` inline comments. RuboCop supports disabling cops for specific lines or ranges via inline comments. rblint currently ignores these directives, causing false positives on code that legitimately disables a cop. This affects at least 15+ FPs on Discourse (Lint/BooleanSymbol, Lint/Debugger).
