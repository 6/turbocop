!items.include?(x)
^^^^^^^^^^^^^^^^^^ Rails/NegateInclude: Use `exclude?` instead of `!include?`.

!users.include?(current_user)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/NegateInclude: Use `exclude?` instead of `!include?`.

!%w[admin mod].include?(role)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/NegateInclude: Use `exclude?` instead of `!include?`.

# constant path receiver
!Config::MODES.include?(mode)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/NegateInclude: Use `exclude?` instead of `!include?`.

# inside if condition
if !Config::MODES.include?(mode)
   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/NegateInclude: Use `exclude?` instead of `!include?`.
  handle_invalid
end

# inside elsif condition
if x.nil?
  handle_nil
elsif !Config::MODES.include?(mode)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/NegateInclude: Use `exclude?` instead of `!include?`.
  handle_invalid
end
