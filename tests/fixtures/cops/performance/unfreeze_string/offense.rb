String.new
^^^^^^^^^^ Performance/UnfreezeString: Use unary plus to get an unfrozen string literal.
String.new('')
^^^^^^^^^^^^^^ Performance/UnfreezeString: Use unary plus to get an unfrozen string literal.
x = String.new
    ^^^^^^^^^^ Performance/UnfreezeString: Use unary plus to get an unfrozen string literal.
String.new('hello')
^^^^^^^^^^^^^^^^^^^ Performance/UnfreezeString: Use unary plus to get an unfrozen string literal.
String.new("hello #{name}")
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/UnfreezeString: Use unary plus to get an unfrozen string literal.
