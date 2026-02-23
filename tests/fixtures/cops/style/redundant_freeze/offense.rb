CONST = 1.freeze
        ^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

CONST2 = 1.5.freeze
         ^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

CONST3 = :sym.freeze
         ^^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.

CONST4 = true.freeze
         ^^^^ Style/RedundantFreeze: Do not freeze immutable objects, as freezing them has no effect.
