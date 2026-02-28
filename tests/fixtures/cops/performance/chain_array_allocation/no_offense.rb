arr.compact
arr.map { |x| x.to_s }
arr.compact.first
arr.flatten.each { |x| x }
arr.sort.select { |x| x.valid? }
arr.compact!.map { |x| x }
arr.lazy.map(&:some_obj_method).reject(&:nil?).first
arr.flatten.flat_map { |x| x }
arr.uniq.flat_map { |x| x }
requirements.flatten.flat_map { |r| r.split(",").map(&:strip) }
[1, 2, 3, 4].first.uniq
model.select(:foo, :bar).select { |item| item.do_something }
model.select(:foo, :bar).select(:baz, :qux)
arr.zip { |f| f }.uniq
# safe navigation chains — RuboCop only matches `send`, not `csend`
items&.select { |x| x.valid? }&.map(&:name)
items&.compact&.map(&:to_s)
records&.map(&:id)&.compact
account.users.where(auto_offline: false)&.map(&:user_id)&.map(&:to_s)
# block_pass inner should not trigger select-as-outer (not any_block_type?)
items.select(&:valid?).select { |x| x.ready? }
items.reject(&:blank?).select { |x| x.present? }
