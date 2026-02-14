Foo.ancestors.include?(Bar)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/AncestorsInclude: Use `is_a?` instead of `ancestors.include?`.
Class.ancestors.include?(Kernel)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/AncestorsInclude: Use `is_a?` instead of `ancestors.include?`.
ancestors.include?(Klass)
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/AncestorsInclude: Use `is_a?` instead of `ancestors.include?`.
object_one.ancestors.include?(object_two)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/AncestorsInclude: Use `is_a?` instead of `ancestors.include?`.
