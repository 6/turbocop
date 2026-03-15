if day.is? :tuesday && month == :jan
   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RequireParentheses: Use parentheses in the method call to avoid confusion about precedence.
  foo
end

day_is? 'tuesday' || true
^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RequireParentheses: Use parentheses in the method call to avoid confusion about precedence.

wd.include? 'tuesday' && true
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RequireParentheses: Use parentheses in the method call to avoid confusion about precedence.

puts has_sync && has_buffer ? "updated" : "missing"
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RequireParentheses: Use parentheses in the method call to avoid confusion about precedence.
