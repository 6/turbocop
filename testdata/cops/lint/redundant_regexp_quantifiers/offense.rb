foo = /(?:a+)+/
      ^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `+` and `+` with a single `+`.
foo = /(?:a*)*/
      ^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `*` and `*` with a single `*`.
foo = /(?:a?)?/
      ^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `?` and `?` with a single `?`.
foo = /(?:a+)?/
      ^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `+` and `?` with a single `*`.
