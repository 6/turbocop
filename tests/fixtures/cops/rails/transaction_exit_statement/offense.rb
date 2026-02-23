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
