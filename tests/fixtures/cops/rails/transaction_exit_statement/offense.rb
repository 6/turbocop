ActiveRecord::Base.transaction do
  return if user.nil?
  ^^^^^^^^^^^^^^^^^^^ Rails/TransactionExitStatement: Do not use `return` inside a transaction block.
  user.save!
end

Account.transaction do
  break if account.closed?
  ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/TransactionExitStatement: Do not use `break` inside a transaction block.
  account.update!(balance: 0)
end

Order.transaction do
  throw :abort if order.invalid?
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/TransactionExitStatement: Do not use `throw` inside a transaction block.
  order.save!
end

# with_lock is also a transaction method
user.with_lock do
  throw if user.active?
  ^^^^^ Rails/TransactionExitStatement: Do not use `throw` inside a transaction block.
end

ApplicationRecord.with_lock do
  break if record.stale?
  ^^^^^^^^^^^^^^^^^^^^^^ Rails/TransactionExitStatement: Do not use `break` inside a transaction block.
end

# return in rescue inside transaction block
ApplicationRecord.transaction do
rescue
  return do_something
  ^^^^^^^^^^^^^^^^^^^ Rails/TransactionExitStatement: Do not use `return` inside a transaction block.
end

# return outside rescue but with rescue present
ApplicationRecord.transaction do
  return if user.active?
  ^^^^^^^^^^^^^^^^^^^^^^ Rails/TransactionExitStatement: Do not use `return` inside a transaction block.
rescue
  pass
end

# return inside loop inside transaction (loop does not break out of outer method)
ApplicationRecord.transaction do
  loop do
    return if condition
    ^^^^^^^^^^^^^^^^^^^ Rails/TransactionExitStatement: Do not use `return` inside a transaction block.
  end
end

# throw inside loop inside transaction
ApplicationRecord.transaction do
  loop do
    throw if condition
    ^^^^^ Rails/TransactionExitStatement: Do not use `throw` inside a transaction block.
  end
end

# Pattern from corpus: return inside transaction inside method with rescue
def call
  ActiveRecord::Base.transaction do
    return false unless place
    ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/TransactionExitStatement: Do not use `return` inside a transaction block.
  end
rescue ActiveRecord::RecordInvalid => e
  handle_error(e)
end

# Pattern from corpus: with_lock without receiver
def refresh_token
  with_lock do
    return unless should_refresh?
    ^^^^^^ Rails/TransactionExitStatement: Do not use `return` inside a transaction block.
    perform_token_refresh!
  end
end
