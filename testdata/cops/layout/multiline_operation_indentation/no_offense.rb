x = 1 +
  2

y = 3 + 4

z = a &&
  b

# Chained || on continuation line (both on same line = no offense)
def related_to_local_activity?
  fetch? || followed_by_local_accounts? || requested_through_relay? ||
    responds_to_followed_account? || addresses_local_accounts?
end

# Multiline block result + operator on same line
x = if true
  begin
    foo
  end + bar
end

# Nested && inside || (right operand of nested op aligned differently)
def acceptable?(node)
  src = node.source
  src.include?(QUOTE) &&
    (STRING_INTERPOLATION_REGEXP.match?(src) ||
    (node.str_type? && double_quotes_required?(src)))
end

# Leading operator style: && at start of continuation line
def regexp_first_argument?(send_node)
  send_node.first_argument&.regexp_type? \
    && REGEXP_ARGUMENT_METHODS.include?(send_node.method_name)
end
