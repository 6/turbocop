Rails.logger.debug "The time is #{Time.zone.now}."
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EagerEvaluationLogMessage: Pass a block to `Rails.logger.debug`.
Rails.logger.debug "User #{user.name} logged in"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EagerEvaluationLogMessage: Pass a block to `Rails.logger.debug`.
Rails.logger.debug "Count: #{items.count}"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EagerEvaluationLogMessage: Pass a block to `Rails.logger.debug`.

# When the debug call is inside a multi-statement block that is itself the sole
# statement of an outer block, it should still be flagged. The sole_block_stmt
# flag must be reset when entering a nested block with multiple statements.
items.each do |item|
  Post.transaction do
    Rails.logger.debug "Processing #{item.name}"
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EagerEvaluationLogMessage: Pass a block to `Rails.logger.debug`.
    do_something(item)
  end
end

# Debug call NOT the sole stmt in its block — flagged
items.each do |item|
  Rails.logger.debug "Processing #{item.name}"
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EagerEvaluationLogMessage: Pass a block to `Rails.logger.debug`.
  process(item)
end

# Debug call inside a conditional that is the sole stmt of a block — should fire
# because debug's parent is IfNode (not block_type), matching RuboCop's semantics.
items.each do |item|
  if item.valid?
    Rails.logger.debug "Valid: #{item.name}"
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EagerEvaluationLogMessage: Pass a block to `Rails.logger.debug`.
  end
end
