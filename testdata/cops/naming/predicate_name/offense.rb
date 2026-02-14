def has_value?
    ^^^^^^^^^^ Naming/PredicateName: Rename `has_value?` to `value?`.
  !@value.nil?
end

def is_valid
    ^^^^^^^^ Naming/PredicateName: Rename `is_valid` to `valid`.
  @valid
end

def has_children?
    ^^^^^^^^^^^^^ Naming/PredicateName: Rename `has_children?` to `children?`.
  @children.any?
end
