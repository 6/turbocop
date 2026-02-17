do_something if condition if other_condition
                          ^^ Style/IfUnlessModifierOfIfUnless: Avoid modifier `if` after another conditional.

do_something unless condition if other_condition
                              ^^ Style/IfUnlessModifierOfIfUnless: Avoid modifier `if` after another conditional.

do_something if a unless b
                  ^^^^^^ Style/IfUnlessModifierOfIfUnless: Avoid modifier `unless` after another conditional.
