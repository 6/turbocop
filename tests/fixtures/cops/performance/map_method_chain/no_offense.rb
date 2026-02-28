x.map { |e| e.foo.bar }
x.map(&:foo).select(&:bar)
x.map(&:foo)
arr.map(&:foo).flat_map(&:bar)
arr.select(&:valid?).map(&:name)
# Chained map with blocks, not symbol args — not flagged
arr.map { |x| x.split('=', 2) }.map { |k, v| [k.downcase, v] }
# flat_map then map is not flagged
arr.flat_map(&:foo).map(&:bar)
# Chain with non-map between
arr.map(&:foo).compact.map(&:bar)
# Multi-line with backslash continuation and non-map calls between
result = items\
  .select(&:active?)\
  .map(&:name)
# Safe navigation on both calls — RuboCop only fires via on_send (not csend)
items&.map(&:name)&.map(&:to_s)
account.users.where(active: true)&.map(&:user_id)&.map(&:to_s)
# RuboCop skips when receiver of chain start has symbol block_pass (non-map method)
items.select(&:active).map(&:name).map(&:to_s)
items.flat_map(&:children).map(&:name).map(&:to_s)
records.reject(&:blank?).map(&:id).map(&:to_s)
