Rails.logger.debug "The time is #{Time.zone.now}."
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EagerEvaluationLogMessage: Pass a block to `Rails.logger.debug`.
Rails.logger.debug "User #{user.name} logged in"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EagerEvaluationLogMessage: Pass a block to `Rails.logger.debug`.
Rails.logger.debug "Count: #{items.count}"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EagerEvaluationLogMessage: Pass a block to `Rails.logger.debug`.
