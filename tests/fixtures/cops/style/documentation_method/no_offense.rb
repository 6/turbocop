# Public method
def foo
  puts 'bar'
end

def initialize
  @x = 1
end

# Another documented method
def bar
  42
end

# Private methods don't need docs (default RequireForNonPublicMethods: false)
private

def private_method
  42
end

protected

def protected_method
  42
end

# Inline private
private def inline_private
  42
end

# Documentation for modular method
module_function def modular_method
  42
end

# Documentation for keywords method
ruby2_keywords def keyword_method
  42
end

# private_class_method is non-public, skipped by default
private_class_method def self.secret
  42
end

# TODO: fix this
# Real documentation follows the annotation
def annotated_then_doc
  42
end

# Private with indented def (common Ruby style)
class IndentedPrivate
  private
    def indented_private_method
      42
    end

  protected
    def indented_protected_method
      42
    end
end

# Private inside class << self followed by private section
module ActionCable
    class Base
      class << self
      end
      private
        def delegate_connection_identifiers
          42
        end
    end
end

# Private in nested class with different indentation
class Container
  class Nested
    private
      def deeply_nested_private
        42
      end
  end
end

# Retroactive private :method_name makes method non-public (no docs needed)
class RetroactivePrivate
  def secret_method
    42
  end
  private :secret_method
end

# Retroactive protected :method_name makes method non-public
class RetroactiveProtected
  def guarded_method
    42
  end
  protected :guarded_method
end

# Multiple methods made private retroactively
class MultiRetroactive
  def helper_one
    42
  end

  def helper_two
    42
  end
  private :helper_one, :helper_two
end

# Retroactive private with string argument
class RetroactivePrivateString
  def string_method
    42
  end
  private "string_method"
end

# public re-establishes visibility after private section
class PublicAfterPrivate
  private

  def secret
    42
  end

  public

  # Documented public method after public keyword
  def visible
    42
  end
end

# Nested class between private and def should not reset visibility
class NestedClassAfterPrivate
  private

  class Inner
    # Documented inner method
    def inner_method
      42
    end
  end

  def still_private_method
    42
  end
end

# Nested module between private and def should not reset visibility
class NestedModuleAfterPrivate
  private

  module Helper
  end

  def also_private_method
    42
  end
end

# Private with trailing whitespace on the private line
class TrailingWhitespacePrivate
  private

  def trailing_ws_method
    42
  end
end

# private(def ...) makes the method private
private(def paren_private_method
  42
end)

# protected(def ...) makes the method protected
protected(def paren_protected_method
  42
end)

# Single-line class defs should not break peer scope tracking
class Webfinger
  class Error < StandardError; end
  class GoneError < Error; end
  class RedirectError < Error; end

  # Documented public method
  def perform
    42
  end

  private

  def secret_helper
    42
  end

  def another_helper
    42
  end
end

# Migration-style: single-line class + private section
class BackfillMigration
  class Account < ActiveRecord::Base; end
  class User < ActiveRecord::Base; end
  class Status < ActiveRecord::Base; end

  # Documented up method
  def up
    process_logs
  end

  private

  def process_logs
    42
  end

  def process_users
    42
  end
end

# Comment with <<WORD before private should not break visibility tracking
class CommentWithHeredocSyntax
  # This comment mentions <<EOF heredoc syntax
  private

  def method_after_comment_with_heredoc
    42
  end
end

# Trailing comment with <<WORD should not trigger heredoc tracking
class TrailingCommentHeredoc
  x = 1 # use <<HEREDOC for multiline
  private

  def method_after_trailing_comment
    42
  end
end

# Line starting with # that has <<WORD should not trigger heredoc
class CommentLineHeredoc
  # Heredocs use <<~RUBY or <<-SQL syntax
  private

  def method_after_comment_line
    42
  end
end

# rubocop:disable directive between doc comment and def should not suppress docs.
# RuboCop sees the doc comment above the blank line via ast_with_comments.

# Create a meaningful operation name from the semantic convention
# @see https://opentelemetry.io/docs/specs/semconv/general/trace/

# rubocop:disable Metrics/CyclomaticComplexity,Metrics/PerceivedComplexity
def method_with_rubocop_disable_after_doc
  42
end

# Converts a value from database input to the appropriate ruby type.
#
# @param value [String] value to deserialize
#
# @return [Object] deserialized value

# rubocop:disable Style/RescueModifier
def method_with_rubocop_disable_after_yard_doc
  42
end

# It accepts an array of coordinates
# and an optional radius

# rubocop:disable Metrics/MethodLength
def method_with_rubocop_disable_after_short_doc
  42
end

# Inline prefix containing `private` makes method non-public (no docs needed)
memoized internal private def memoized_private_method
  42
end

# protected in a mixed inline prefix also makes method non-public
some_decorator protected def decorated_protected_method
  42
end
