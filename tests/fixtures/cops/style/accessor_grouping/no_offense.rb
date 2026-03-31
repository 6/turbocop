class Foo
  attr_reader :bar1, :bar2, :bar3

  attr_accessor :quux

  attr_writer :baz
end

class Bar
  attr_reader :x
end

# Accessors separated by annotation method calls - not grouped
class WithAnnotations
  extend T::Sig

  annotation_method :one
  attr_reader :one

  annotation_method :two
  attr_reader :two
end

# Accessor preceded by a comment on the previous line - excluded from grouping
class WithComments
  # @return [String] value of foo
  attr_reader :one, :two

  attr_reader :four
end

# Accessors in different visibility scopes - not grouped across scopes
class WithScopes
  attr_reader :public_one

  private

  attr_reader :private_one
end

# Sorbet sig block makes accessor not groupable (no blank line after sig)
class WithSorbet
  extend T::Sig

  sig { returns(Integer) }
  attr_reader :one

  attr_reader :two, :three
end

# Accessor preceded by a comment on the line before
class CommentBeforeAccessor
  # This is a comment about alpha
  attr_reader :alpha

  # This is a comment about beta
  attr_reader :beta
end

# Single accessor per type in each visibility scope
class SinglePerScope
  attr_reader :a

  private

  attr_reader :b

  protected

  attr_reader :c
end

# Accessors with Sorbet annotations (no blank line gap) - not groupable
class SorbetAnnotated
  extend T::Sig

  annotation_method :one
  attr_reader :one

  annotation_method :two
  attr_reader :two

  sig { returns(Integer) }
  attr_reader :three
end

# Accessors with RBS::Inline annotations are not groupable
class WithRBSAnnotations
  attr_accessor :label #: String
  attr_accessor :points #: Array[Float | Integer]
  attr_accessor :color #: String
end

# RBS::Inline annotations with blank lines between accessors
class WithRBSSpaced
  attr_reader :reader #: Reader

  attr_reader :writer #: Writer

  attr_reader :current_dir #: Pathname
end

# Mixed: one accessor with RBS annotation, others without - no grouping
# because the previous sibling has an inline #: comment
class WithRBSMixed
  attr_reader :one #: String

  attr_reader :two, :three
end

# Bare accessor calls with no arguments (e.g. used as annotations) - not grouped
class BareAttr
  attr
  def label
    @object.attributes['name']
  end

  attr
  def value
    @object.attributes['_id']
  end
end
