str.downcase == "other"
^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `downcase ==`.
str.upcase == "OTHER"
^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `upcase ==`.
str.downcase == 'string'
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `downcase ==`.
str.upcase == 'string'
^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `upcase ==`.

# != operator
str.downcase != 'string'
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `downcase !=`.
str.upcase != 'string'
^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `upcase !=`.

# Reversed operand order (string literal on LHS)
"english" == system.downcase
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `== downcase`.
'CONTENT-TYPE' == key.upcase
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `== upcase`.

# eql? method
str.downcase.eql? "foo"
^^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `downcase eql?`.
str.upcase.eql?('BAR')
^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `upcase eql?`.

# downcase == downcase
str.downcase == str.downcase
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `downcase ==`.
str.upcase == other.upcase
^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `upcase ==`.

# Parenthesized string
str.downcase == ("foo")
^^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `downcase ==`.
