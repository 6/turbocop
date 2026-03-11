User.all.find_each { |u| u.save }
[1, 2, 3].each { |x| puts x }
users.each { |u| u.save }
User.find_each { |u| u.update(active: true) }
records.map { |r| r.name }

# select is not an AR scope method — don't flag Dir.entries().select().each
Dir.entries(dir).select { |f| f.match?(/\.rb/) }.each { |f| puts f }
items.select { |i| i.valid? }.each { |i| process(i) }

# Safe navigation — &.where(&.each) should not be flagged
records&.where(status: :pending)&.each(&:process!)

# No-receiver scope calls in non-AR classes should not be flagged
class Processor
  all.each { |u| u.x }
end
class Worker < Foo
  all.each { |u| u.x }
end

# model.errors.where should not be flagged (Active Model Errors, not AR)
model.errors.where(:title).each { |error| do_something(error) }

# AllowedMethods default: order and limit should not be flagged
User.order(:name).each { |u| u.something }
User.all.order(:name).each { |u| u.something }
User.limit(10).each { |u| u.something }
User.all.limit(10).each { |u| u.something }

# AllowedMethods anywhere in the chain should suppress offense
User.order(:name).includes(:company).each { |u| u.something }

# Constants without scope methods should not be flagged
FOO.each { |u| u.x }

# model.errors.where inside a class should not be flagged
class Model < ApplicationRecord
  model.errors.where(:title).each { |error| do_something(error) }
end

# select/limit/order in the chain should suppress offense
User.all.select(:name, :age).each { |u| u.something }
User.where(active: true).limit(10).each { |u| u.something }
User.where(active: true).select(:name).each { |u| u.something }

# lock in the chain should suppress offense (in default AllowedMethods)
User.lock.each { |u| u.something }
User.where(active: true).lock.each { |u| u.something }

# select in the chain should suppress offense (in default AllowedMethods)
User.select(:name).each { |u| u.something }

# No-receiver in non-class context should not be flagged
all.each { |u| u.x }
where(name: name).each { |u| u.x }

# AllowedPatterns should use regex matching, not substring
# (a config with AllowedPatterns: ['order'] should match 'order' as regex)

# select in a chain before an AR scope method should suppress (in default AllowedMethods)
User.select(:name).where(active: true).each { |u| u.something }

# lock in a chain before an AR scope method should suppress (in default AllowedMethods)
User.lock.where(active: true).each { |u| u.something }
