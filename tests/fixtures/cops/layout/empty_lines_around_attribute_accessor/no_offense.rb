class Foo
  attr_accessor :foo

  def do_something
  end
end

class Bar
  attr_accessor :foo
  attr_reader :bar
  attr_writer :baz

  def example
  end
end

class Baz
  attr_accessor :foo
  alias :foo? :foo

  def example
  end
end

# YARD-documented attribute accessors with comments between them
class ExecutionResult
  # @return [Object, nil]
  attr_reader :value
  # @return [Exception, nil]
  attr_reader :handled_error
  # @return [Exception, nil]
  attr_reader :unhandled_error

  def example
  end
end

# attr_reader inside if/else branch — no offense (RuboCop skips if_type? parents)
if condition
  attr_reader :foo
else
  do_something
end

# attr_reader inside if/elsif branch
if condition
  attr_reader :foo
elsif other_condition
  do_something
end

# attr_writer inside case/when
case x
when :a
  attr_writer :foo
when :b
  do_something
end

# attr_accessor inside begin/rescue
begin
  attr_accessor :foo
rescue StandardError
  handle_error
end

# attr_reader inside begin/ensure
begin
  attr_reader :foo
ensure
  cleanup
end

# attr_accessor followed by else
if something
  attr_accessor :bar
else
  other_thing
end

# attr_accessor inside unless
unless condition
  attr_accessor :baz
else
  fallback
end

# attr_reader followed by whitespace-only blank line (spaces, visually blank)
class WhitespaceBlankLine
  attr_reader :if_condition
    
  # The condition that must *not* be met on an object
  attr_reader :unless_condition

  def example
  end
end

# attr calls used as expressions inside parentheses — not standard accessors
Class.new do
  (attr :foo, 'bar').should == [:foo, :bar]
  (attr :baz, false).should == [:baz]
  (attr :qux, true).should == [:qux, :qux=]
end

# attr call inside an array literal expression — not an accessor statement
values = [
  attr(:greeting, call(:concat, lit("Hello, "), field_ref(:name)))
]

# attr inside single-line block braces
-> { Class.new { attr :foo } }.should raise_error(TypeError)
mod.module_eval { attr_reader(:name) }
assert_raise(NameError) { mod.module_eval { attr(name) } }

# attr_reader inside single-line block with variable
-> { Class.new { attr_reader o } }.should raise_error(TypeError)

# attr_accessor as the last statement in a block (no right sibling)
Class.new do
  attr_accessor :foo
end

# attr_reader as the only statement in a class
class OnlyAttr
  attr_reader :bar
end

# attr_accessor inside Class.new { } followed by }.new — block closing
result = define_class("ResultInstance") {
  attr_accessor :id, :created_at
}.new

# attr_reader inside Class.new do...end followed by end.new
result = Class.new do
  attr_reader :bar
end.new

# attr_accessor inside Class.new { } followed by }.new with method chain
result = Class.new {
  attr_accessor :name
}.new.freeze

# multi-line attr_accessor with comma continuation and blank lines between args
class Config
  attr_accessor :username_attribute_names,           # first attribute
                                                     # as the login.

                :password_attribute_name,           # second attribute
                                                     # for encryption.

                :email_attribute_name              # third attribute
end

# multi-line attr_accessor followed by end (inside class_eval block)
base.sorcery_config.class_eval do
  attr_accessor :token_attribute_name, # token attribute name.
                :expiry_attribute_name # expiry attribute name.
end

# attr_accessor with splat argument followed by comment then alias
attr_accessor(*VALID_OPTIONS_KEYS)
# @private
alias auth_token= private_token=

# attr_accessor with conditional modifier (unless) — next line is code
def new(*)
  attr_accessor :parser unless method_defined? :parser
  result        = super
  result.parser = OptionParser.new
  result
end

# attr_reader with conditional modifier (unless) and method call arg
def offset(*keys)
  keys.each do |key|
    attr_reader key unless method_defined?(method_name(key))
    define_method :"#{key}=" do |value|
    end
  end
end

# attr_reader with variable argument followed by comment then allowed method
attr_reader(attrb.name)
# compatibility fix
public(attrb.name)

# attr_reader followed by comment then blank line (no offense needed)
class ChordQuality
  attr_reader :name
  # QUALITIES_FILE = File.expand_path("qualities.json", __FILE__)

  private

  def something
  end
end

# long single-line attr_reader followed by comment then blank line
class Services
  attr_reader :accounts
  attr_reader :account_links, :sessions, :domains, :fees, :balance, :charges
  # end of generated section

  attr_reader :oauth

  def initialize
  end
end

# attr_reader inside Module.new do...end) block argument — no offense
# The attr is the only/last statement in the block body (no right sibling)
body.extend(Module.new do
  attr_reader :buffer
end)

# attr_accessor inside Module.new do...end) with include
handler.extend(Module.new do
  include SomeModule
  attr_accessor :session, :channel
end)

# attr_reader inside block with end) followed by code on next line
body.extend(Module.new do
  attr_reader :buffer
end)
assert body.buffer.nil?

# attr_accessor without space before colon arg (attr_accessor:name)
class JoinPipe
  attr_reader :block, :groups, :unique
  attr_accessor:to_emit

  def initialize
  end
end

# attr_accessor followed by rubocop:enable directive then blank line — no offense
# RuboCop treats enable directive + blank line as a valid separator
class ContentPage
  # rubocop:disable Lint/DuplicateMethods
  attr_accessor :content_html
  # rubocop:enable Lint/DuplicateMethods

  def url
  end
end

# attr_accessor as the last statement before same-line `end`
class InlineAttr; attr_accessor :stackoff; end

# inline comment containing `if` must not break accessor grouping
class DisasmWidget
  attr_accessor :entrypoints, :gui_update_counter_max
  attr_accessor :keyboard_callback, :keyboard_callback_ctrl # hash key => lambda { |key| true if handled }
  attr_accessor :clones

  def example
  end
end

# DSL methods named `attr` inside branches are not attribute accessor statements
module DryCrud
  module Table
    module Sorting
      def sortable_attr(attr, header = nil, &block)
        if template.sortable?(attr)
          attr(attr, sort_header(attr, header), &block)
        else
          attr(attr, header, &block)
        end
      end
    end
  end
end

# wrapper methods that forward to `attr` are also not accessors
module StandardTableBuilder
  module Sorting
    def sortable_attr(a, header = nil, &block)
      attr(a, sort_header(a, header), &block)
    end
  end
end
