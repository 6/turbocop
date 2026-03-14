x.present?
x.blank?
!x.empty?
x.nil?
name.present? && name.valid?

# unless blank? with else clause should NOT be flagged
# (Style/UnlessElse handles these; RuboCop skips them)
unless foo.blank?
  do_something
else
  do_other
end

unless user.name.blank?
  greet(user)
else
  ask_name
end

# Safe navigation with blank? should NOT be flagged
# (RuboCop's `send` does not match `csend`/safe navigation)
!foo&.blank?
!user.name&.blank?

# Safe navigation in unless blank? should NOT be flagged
something unless foo&.blank?
unless record&.blank?
  do_something
end

# Safe navigation in empty? on the right side of && should NOT be flagged
foo && !foo&.empty?
!foo.nil? && !foo&.empty?

# Different receivers in && patterns should NOT be flagged
!x.nil? && !y.empty?
foo != nil && !bar.empty?
!!x && !y.empty?
a && !b.empty?

# blank? called with argument (class method style, e.g. Helpers.blank?(value)) should NOT be flagged
# RuboCop's NodePattern `(send $_ :blank?)` requires blank? to have no arguments
!Helpers.blank?(value)
!SomeModule::Helpers.blank?(options[:key])
unless Helpers.blank?(value)
  do_something
end
