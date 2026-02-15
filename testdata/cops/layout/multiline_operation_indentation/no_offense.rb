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
