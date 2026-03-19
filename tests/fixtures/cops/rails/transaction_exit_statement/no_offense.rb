ActiveRecord::Base.transaction do
  raise ActiveRecord::Rollback if user.nil?
  user.save!
end

ActiveRecord::Base.transaction do
  User.create!(name: "test")
end

# next is fine (commits the transaction)
ApplicationRecord.transaction do
  next if user.active?
end

# raise is fine (triggers rollback explicitly)
ApplicationRecord.transaction do
  raise "User is active" if user.active?
end

# break inside loop inside transaction is fine (breaks loop, not transaction)
ApplicationRecord.transaction do
  loop do
    break if condition
  end
end

# break inside while inside transaction is fine
ApplicationRecord.transaction do
  while proceed_looping? do
    break if condition
  end
end

# break inside until inside transaction is fine
ApplicationRecord.transaction do
  until stop_looping? do
    break if condition
  end
end

# break inside each inside transaction is fine
ApplicationRecord.transaction do
  records.each do |record|
    break if record.nil?
  end
end

# empty transaction block
ApplicationRecord.transaction do
end

# not a transaction method
other_method do
  return if condition
end

# method call chained (no block)
transaction.foo

# safe navigation &.with_lock — RuboCop's on_send doesn't fire for csend (safe navigation)
# so return inside &.with_lock should NOT be flagged
prefix_val&.with_lock do |factory|
  return yield(factory) unless prefix_val.deleted?
end
