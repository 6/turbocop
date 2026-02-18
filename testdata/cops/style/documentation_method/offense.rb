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
