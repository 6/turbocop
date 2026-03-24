class Foo
  include Bar

  attr_reader :baz
end

class Baz
  extend ActiveSupport::Concern
  include Enumerable
  prepend MyModule

  def some_method
  end
end

class Simple
  include Comparable
end

# include inside a block as sole statement is not flagged
RSpec.describe User do
  include ActiveJob::TestHelper
end

# include inside a block with empty line after is fine
RSpec.describe User do
  include ActiveJob::TestHelper

  let(:user) { create(:user) }
end

# comment between includes does not trigger offense
class UserModel
  include Avatarable
  # Include default devise modules.
  include DeviseTokenAuth::Concerns::User
  include Devise::Models::Confirmable

  attr_reader :name
end

# include used as RSpec matcher argument
it "includes the item" do
  expect(result).to include(item)
end

# include in a single-line class body
class InlineWidget; include Comparable; end

# include followed by a block close continuation
builder = Class.new do
  include AccountableConcern
end.new

# grouped includes followed by a block close
tests(Module.new {
  include LegacyTagHelpers
  include ModernTagHelpers
})

# include before else is allowed
if feature_enabled?
  include FeatureSupport
else
  disable_feature
end

# include inside if/else branches (RuboCop skips when parent is if_type?)
class Account
  if condition
    include Bar
  else
    do_something
  end
end

# include inside unless
class Report
  unless disabled?
    include Logging
  end
end

# include inside if with elsif
class Widget
  if rails?
    include ActiveModel::Validations
  elsif sinatra?
    include SinatraHelper
  else
    include BasicHelper
  end
end

# include with modifier form followed by another include
class Service
  include Bar
  include Baz if condition
  include Qux
end

# include at top level inside if
if condition
  include Foo
else
  do_something
end

# include followed by whitespace-only line (should be treated as blank)
class WithTrailingSpaces
  include Comparable
    
  def compute
  end
end

# extend followed by whitespace-only line
module WithTabs
  extend ActiveSupport::Concern

  def setup
  end
end

# extend inside block followed by receiver.extend (not a module inclusion line)
def make_class
  Class.new do
    extend ClassMethods
    singleton_class.extend(ClassMethods)
    singleton_class.send(:define_method, :run) { true }
  end
end

# prepend followed by singleton_class.prepend
class Util
  prepend CoreExt::PrivateHelpers
  singleton_class.prepend CoreExt::PublicHelpers

  def compute
  end
end

# include followed by rubocop enable directive (with space) then blank line
# rubocop: disable Style/MixinUsage
include Protocol
include Protocol::Header
include Protocol::Error
# rubocop: enable Style/MixinUsage

records = build_records

# include used as receiver in method chain (RSpec matcher)
def expected_items
  include(first_item.uri, second_item.uri)
    .and(not_include(excluded_item.uri))
end

# include inside multi-statement if followed by empty line (no offense)
class Config
  if RUBY_VERSION >= '1.9'
    include Comparable

    def <=>(other)
      name <=> other.name
    end
  end
end

# include/extend inside begin/rescue — rescue terminates the block
class Generator
  def initialize
    begin
      mixin = self.class.const_get(name)
      extend mixin
    rescue NameError
      raise ArgumentError, "Unsupported"
    end
  end
end

# include inside begin/rescue at class level
begin
  require 'optional_dep'
  include REXML
rescue LoadError
  puts "missing gem"
end

# single-line class with semicolons followed by blank line
class Wrapper; extend Mixin; run_setup; end

class OtherWrapper; extend Mixin; run_setup; end

module Lexer
  prepend :classname do
    rule %r/x/, Text
  end

  prepend :funcname do
    rule %r/x/, Text
  end
  # This is a fix for the way that function calls are lexed in the Python
  # lexer. This should be moved to the Python lexer once confirmed that it
  # does not cause any regressions.
  state :func_call_fix do
  end
end
