def foo
^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
  puts 'bar'
end

def method; end
^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.

def another_method
^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
  42
end

# TODO: fix this later
def annotated_method
^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
  42
end

# rubocop:disable Style/Foo
def directive_method
^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
  42
end

# frozen_string_literal: true
def interpreter_directive_method
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
  42
end

module_function def undocumented_modular
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
  42
end

# Documentation above the line is for the wrapping call, not the def
memoize def memoized_method
        ^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
  42
end

# Outputs an element tag.
register_element def custom_tag(**attrs, &content) = nil
                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.

module Postmark
  module HashHelper
    # Compatibility shim
    def enhance_with_compatibility_warning(hash)
      def hash.[](key)
      ^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
        42
      end
    end
  end
end

class UpdateChecker
  # Returns the update check service.
  def update_check_service
    Struct.new(:origin) do
      def latest_version
      ^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
        42
      end
    end
  end
end

module Datadog
  module LibdatadogExtconfHelpers
    # Note: This helper is currently only used in the `libdatadog_api/extconf.rb`
    def self.load_libdatadog_or_get_issue
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
      42
    end
  end
end

class Sender
  private

  if CLOSEABLE_QUEUES
    def send_loop
    ^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
      42
    end
  else
    def send_loop
    ^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
      42
    end
  end
end

class StatSerializer
  private

  if RUBY_VERSION < '3'
    def metric_name_to_string(metric_name)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
      metric_name.to_s
    end
  else
    def metric_name_to_string(metric_name)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
      metric_name.to_s
    end
  end
end

if FEATURE_AVAILABLE
  def conditional_method
  ^^^^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
    42
  end
end

# Documentation above the line is for the wrapping modifier, not the def
def rdoc_dummy_method; super; end if false
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.

class MultiRetroactiveProtected
  def helper_one
  ^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
    42
  end

  def helper_two
  ^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
    42
  end
  protected :helper_one, :helper_two
end

class RetroactivePrivateString
  def string_method
  ^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
    42
  end
  private "string_method"
end

protected (def spaced_paren_protected
           ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DocumentationMethod: Missing method documentation comment.
  42
end)

def json!
^ Style/DocumentationMethod: Missing method documentation comment.

def articles_courses_scope
^ Style/DocumentationMethod: Missing method documentation comment.

def scope
^ Style/DocumentationMethod: Missing method documentation comment.
