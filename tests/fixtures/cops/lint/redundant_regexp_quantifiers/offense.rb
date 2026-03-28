foo = /(?:a+)+/
      ^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `+` and `+` with a single `+`.
foo = /(?:a*)*/
      ^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `*` and `*` with a single `*`.
foo = /(?:a?)?/
      ^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `?` and `?` with a single `?`.
foo = /(?:a+)?/
      ^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `+` and `?` with a single `*`.
foo = /https{0,1}?:/
      ^^^^^^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `{0,1}` and `?` with a single `?`.
foo = /<.{1,}?>/
      ^^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `{1,}` and `?` with a single `*`.
foo = /a{0,}?b/
      ^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `{0,}` and `?` with a single `*`.
src.match?(%r{\A(?:https{0,1}?:)?//player\.example\.com/embed/})
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `{0,1}` and `?` with a single `?`.

if fragment.match(/\A(\w+)=(.?*)\z/) && valid_url?($2)
                  ^ Lint/RedundantRegexpQuantifiers: Replace redundant quantifiers `?` and `*` with a single `*`.
