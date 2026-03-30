foo { do_something(_1, _2) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NumberedParametersLimit: Avoid using more than 1 numbered parameter; 2 detected.

bar { _1 + _2 + _3 }
^^^^^^^^^^^^^^^^^^^^^ Style/NumberedParametersLimit: Avoid using more than 1 numbered parameter; 3 detected.

baz { puts _1, _2, _3, _4, _5 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NumberedParametersLimit: Avoid using more than 1 numbered parameter; 5 detected.

-> {
^ Style/NumberedParametersLimit: Avoid using more than 1 numbered parameter; 2 detected.
  _1 + _2
}

duplicates = mapper_classes.map { [_1.relation, _1.register_as] }.tally.select { _2 > 1 }
             ^ Style/NumberedParametersLimit: Avoid using more than 1 numbered parameter; 2 detected.
