# typed: false
# This file exercises magic-comment handling beyond the first few lines.
# RuboCop accepts both frozen_string_literal and frozen-string-literal.
# frozen-string-literal: true

CONST = 1.freeze
        ^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

CONST2 = 1.5.freeze
         ^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

CONST3 = :sym.freeze
         ^^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

CONST4 = true.freeze
         ^^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

CONST5 = false.freeze
         ^^^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

CONST6 = nil.freeze
         ^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

# Plain string with frozen-string-literal: true is redundant
GREETING = 'hello'.freeze
           ^^^^^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

EMPTY = ''.freeze
        ^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

DOUBLE_QUOTED = "hello world".freeze
                ^^^^^^^^^^^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

class LeagueAdminAiService
  SYSTEM_PROMPT = <<~PROMPT.freeze
    You are an investigation assistant.
  PROMPT
end
# nitrocop-expect: 26:18 Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

module TerraformLandscape
  FALLBACK_MESSAGE = 'Terraform Landscape: a parsing error occured.' \
                     ' Falling back to original Terraform output...'.freeze
end
# nitrocop-expect: 32:21 Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.
