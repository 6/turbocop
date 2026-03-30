class Foo
  attr_accessor :foo
  ^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  def do_something
  end
end

class Bar
  attr_reader :bar
  ^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  def another_method
  end
end

class Baz
  attr_writer :baz
  ^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  def yet_another
  end
end

# attr_accessor followed by YARD comments then blank line then code — offense
# RuboCop flags because no blank line directly after the attr_accessor
class TensorOutput
  attr_accessor :index, :operation
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  # @!attribute index
  # Index specifies the index of the output.
  # @!attribute operation
  # Operation is the Operation that produces this Output.

  def compute
  end
end

# attr_accessor followed by comments then blank line — offense
class SessionConfig
  attr_accessor :status, :graph
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  # @!attribute dimensions
  # Dimensions of the graph.

  def run
  end
end

# attr_reader followed by single comment then code — offense
class CommentThenCode
  attr_reader :value
  ^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  # some comment
  def process
  end
end

# attr_writer followed by multiple comments then code — offense
class MultiCommentThenCode
  attr_writer :data
  ^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  # comment one
  # comment two
  def transform
  end
end

# attr_accessor with trailing semicolon — semicolon is just a statement terminator
class SemicolonAttr
  attr_accessor :foo;
  ^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  def bar; end
end

# attr_reader with trailing semicolon
class SemicolonReader
  attr_reader :closed;
  ^^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  def each; yield('foo'); yield('bar'); end;
end

# attr_accessor followed by alias_method with if modifier — not an allowed successor
class DynamicAttr
  attr_accessor :name
  ^^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  alias_method :other, :name if condition
end

# attr_reader followed by conditional attr_writer — not a true attr successor
def attr(name, writer=false)
  attr_reader name
  ^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  attr_writer name if writer
end

class Cookies<H;attr_accessor :_p
                ^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
def _n =@n||={}
end

module Base;attr_accessor:env,:request,:root,:input,:cookies,:state,:status,
            ^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
:headers,:body,:url_prefix
def lookup; end
end

class DeprecatedBase
  class << self
    attr_accessor :deprecated do
    ^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
      false
    end

    def category
    end
  end
end

case attr 'source-highlighter'
     ^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
when 'coderay'
end

class Configuration
  attr_accessor(:reporter) { AbstractAdapter.new }
  ^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
end

class InstallationOptions
  def self.option(name, default, boolean: true)
    defaults[name] = default
    attr_accessor name
    ^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
    alias_method "#{name}?", name if boolean
  end
end

class InstallationOptionsMirror
  def self.option(name, default, boolean: true)
    defaults[name] = default
    attr_accessor name
    ^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
    alias_method "#{name}?", name if boolean
  end
end

class ConfigurationBuilder
  options.each do |o|
    attr_reader o.name
    ^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
    alias_method :"#{o.name}?", o.name if o.type == BOOLEAN
  end
end
