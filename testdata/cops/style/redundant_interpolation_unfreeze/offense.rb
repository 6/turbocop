+"#{foo} bar"
^ Style/RedundantInterpolationUnfreeze: Don't unfreeze interpolated strings as they are already unfrozen.

"#{foo} bar".dup
             ^^^ Style/RedundantInterpolationUnfreeze: Don't unfreeze interpolated strings as they are already unfrozen.

"#{foo} bar".+@
             ^^ Style/RedundantInterpolationUnfreeze: Don't unfreeze interpolated strings as they are already unfrozen.
