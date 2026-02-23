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
