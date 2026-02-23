return if foo.empty?
foo.empty?
bar.blank?
baz&.present?
qux&.nil?

# &.empty? outside a conditional is not flagged (only if/unless conditions)
x&.empty?
items.delete_if { |e| e.str_content&.empty? }
