# frozen_string_literal: true

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

# Plain string with frozen_string_literal: true is redundant
GREETING = 'hello'.freeze
           ^^^^^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

EMPTY = ''.freeze
        ^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

DOUBLE_QUOTED = "hello world".freeze
                ^^^^^^^^^^^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.
