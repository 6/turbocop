# Method returning comparison should end with ?
def foo
    ^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  bar == baz
end

# Method returning negation should end with ?
def checks_negation
    ^^^^^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  !x
end

# Method returning predicate call should end with ?
def checks_predicate
    ^^^^^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  bar?
end

# Method returning true should end with ?
def returns_true
    ^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  true
end

# Method returning false should end with ?
def returns_false
    ^^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  false
end

# Predicate method returning non-boolean literal
def bad_predicate?
    ^^^^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  5
end

# Predicate method returning string literal
def string_pred?
    ^^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  'hello'
end

# Predicate method returning nil literal
def nil_pred?
    ^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  nil
end

# Class method returning boolean
def self.class_check
         ^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  x > y
end
