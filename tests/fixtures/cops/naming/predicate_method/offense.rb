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

# Predicate with bare return and ||= assignment (assignment is not call_type)
def self.enterprise?
         ^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  return if ENV.fetch('DISABLE_ENTERPRISE', false)
  @enterprise ||= root.join('enterprise').exist?
end

# Predicate with modifier-if assignment as implicit return
def valid_event_payload?
    ^^^^^^^^^^^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  @channel = Channel::Line.find_by(line_channel_id: @params[:line_channel_id]) if @params[:line_channel_id]
end

# Explicit return with compound and-expression (return a? && b?)
def has_flag
    ^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  return active? && enabled?
end

# Explicit return with compound or-expression (return x > 0 || y > 0)
def is_valid
    ^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  return x > 0 || y > 0
end

# Explicit return with case expression
def has_role
    ^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  case kind
  when :admin then admin?
  when :member then member?
  else false
  end
end

# Nested def inside singleton class inside another method
def setup
  class << (@object = Object.new)
    def callback
        ^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
      true
    end
  end
end

# Method ending with ? returning nil from early return, and call+block as implicit return
# In RuboCop, call+block is NOT call_type?, so conservative skip doesn't apply
def fragment_exist?(key, options = nil)
    ^^^^^^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  return unless cache_configured?
  instrument_fragment_cache(:exist_fragment?, key) do
    cache_store.exist?(key, options)
  end
end

# Predicate method returning a variable (not call_type in RuboCop)
# with non-boolean literal in another branch
def instance_type?(type)
    ^^^^^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  if type.is_a?(Types::Name::Instance)
    type
  end
end

# Non-predicate returning block_argument predicate call
def self.auto_bump_topic!
         ^^^^^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  Category.shuffle.any?(&:auto_bump_topic!)
end
