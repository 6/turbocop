do_thing if x if y
         ^^ Style/NestedModifier: Avoid using nested modifiers.

run if a if b
    ^^ Style/NestedModifier: Avoid using nested modifiers.

something if a unless b
          ^^ Style/NestedModifier: Avoid using nested modifiers.

something if a while b
          ^^ Style/NestedModifier: Avoid using nested modifiers.

something if a until b
          ^^ Style/NestedModifier: Avoid using nested modifiers.

something while a if b
          ^^^^^ Style/NestedModifier: Avoid using nested modifiers.

something until a if b
          ^^^^^ Style/NestedModifier: Avoid using nested modifiers.
