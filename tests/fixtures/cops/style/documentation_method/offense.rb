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
