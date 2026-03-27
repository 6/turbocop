class Foo
  attr_reader :bar1
  ^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  attr_reader :bar2
  ^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  attr_accessor :quux
  attr_reader :bar3, :bar4
  ^^^^^^^^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  other_macro :zoo
end

# Accessors separated by method definitions are still groupable
class WithDefs
  def foo
  end
  attr_reader :one
  ^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.

  def bar
  end
  attr_reader :two
  ^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
end

# Non-contiguous accessors separated by blank lines are groupable
class BlankLineSeparated
  attr_reader :alpha
  ^^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.

  attr_reader :beta
  ^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
end

# Accessors after bare visibility modifiers are groupable within scope
class WithVisibility
  attr_reader :pub1
  ^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  attr_reader :pub2
  ^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.

  private
  attr_reader :priv1, :priv2
  ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  attr_writer :w1
  attr_reader :priv3
  ^^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.

  public
  attr_reader :pub3
  ^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  other_macro :zoo
end

# Accessors after annotation method + blank line are still groupable with others
class AfterAnnotation
  extend T::Sig

  sig { returns(Integer) }
  attr_reader :one

  attr_reader :two, :three
  ^^^^^^^^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.

  attr_reader :four
  ^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
end

# Accessors after block-form DSL calls are grouped by the call line, not the block end
class AfterBlockMacro
  mattr_accessor :items do
    []
  end
  attr_reader :name
  ^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  attr_reader :url
  ^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  attr_reader :enabled
  ^^^^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
end

# Block-form config DSLs behave the same way as other block calls
class AfterConfigSection
  config_section :client, param_name: :clients do
    config_param :host, :string, default: nil
  end
  attr_reader :nodes
  ^^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.

  attr_reader :sessions
  ^^^^^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
end
