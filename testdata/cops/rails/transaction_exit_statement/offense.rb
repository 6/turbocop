ActiveRecord::Base.transaction do
  return if user.nil?
  ^^^^^^^^^^^^^^^^^^^ Rails/TransactionExitStatement: Do not use `return` inside a transaction block.
  user.save!
end
