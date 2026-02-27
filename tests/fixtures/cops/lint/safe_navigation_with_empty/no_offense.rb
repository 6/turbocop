return if foo.empty?
foo.empty?
bar.blank?
baz&.present?
qux&.nil?

# &.empty? outside a conditional is not flagged (only if/unless conditions)
x&.empty?
items.delete_if { |e| e.str_content&.empty? }

# Receiver is a local variable — not a send node, should not flag
# (RuboCop pattern requires (csend (send ...) :empty?), not (csend lvar :empty?))
return unless foo&.empty?
bar if baz&.empty?
do_something if x&.empty?
return if variable&.empty?

if items&.empty?
  do_something
end

unless data&.empty?
  process(data)
end

# Receiver is a safe navigation chain — csend, not send
if name&.strip&.empty?
  set_default
end
