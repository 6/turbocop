def has_value?
    ^^^^^^^^^^ Naming/PredicatePrefix: Rename `has_value?` to `value?`.
  !@value.nil?
end

def is_valid
    ^^^^^^^^ Naming/PredicatePrefix: Rename `is_valid` to `valid`.
  @valid
end

def has_children?
    ^^^^^^^^^^^^^ Naming/PredicatePrefix: Rename `has_children?` to `children?`.
  @children.any?
end
