Bad = Struct.new(:members)
                 ^^^^^^^^ Lint/StructNewOverride: `:members` member overrides `Struct#members` and it may be unexpected.

Bad2 = Struct.new(:count)
                  ^^^^^^ Lint/StructNewOverride: `:count` member overrides `Struct#count` and it may be unexpected.

Bad3 = Struct.new(:hash)
                  ^^^^^ Lint/StructNewOverride: `:hash` member overrides `Struct#hash` and it may be unexpected.
