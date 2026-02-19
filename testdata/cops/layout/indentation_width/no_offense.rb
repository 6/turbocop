class Foo
  x = 1
end

def bar
  y = 2
end

if true
  z = 3
end

while true
  a = 1
end

module Baz
  CONST = 1
end

def single_line; end

# Block body indented from line start, not from do/{
items.each do |item|
  process(item)
end

settings index: index_preset(refresh_interval: '30s') do
  field(:id, type: 'long')
end

[1, 2].map { |x|
  x * 2
}

case x
when 1
  do_something
when 2
  do_other
end

# Block on chained method — body indented relative to dot
source.passive_relationships
      .where(account: Account.local)
      .in_batches do |follows|
        process(follows)
      end

# Block body indented from dot when dot is on a new line (matching RuboCop)
source.passive_relationships
      .where(account: Account.local)
      .in_batches do |follows|
        process(follows)
      end

# Chained method with do..end block, body indented from dot
account.conversations
       .joins(:inbox)
       .where(created_at: range)
       .each_with_object({}) do |((channel_type, status), count), grouped|
         grouped[channel_type] ||= {}
         grouped[channel_type][status] = count
       end

# Block with dot NOT on a new line — uses end column as base
items.each do |item|
  process(item)
end

# Assignment context (variable style): body indented from LHS, end at LHS
x = if foo
  bar
end

result = if condition
  value_a
end

y = while queue.any?
  queue.pop
end

z = until done
  process_next
end

# Assignment context (keyword style): body indented from keyword, end at keyword
links = if enabled?
          body
        end

# Inline block wrapping — closing } on same line as body
get "/", constraints: lambda { |req|
  req.subdomain.present? && req.subdomain != "clients" },
           to: lambda { |env| [200, {}, %w{default}] }

# Block params on same line as body
files = (Dir["test/**/*_test.rb"].reject {
  |x| x.include?("/adapters/")
} + Dir["test/other/**/*_test.rb"]).sort

# Multi-line when with `then` on continuation line
case type
when :references, :belongs_to,
     :attachment, :attachments,
     :rich_text                   then nil
when :string
  "MyString"
end

# Misaligned end with body correctly indented from `if` keyword
# (EndAlignment disabled scenario — end at arbitrary column)
x = if foo
      bar
    end

# Misaligned end with body correctly indented from `while` keyword
y = while queue.any?
      queue.pop
    end

# Misaligned end with body correctly indented from `until` keyword
z = until done
      process_next
    end
