items.each do |item|
  return if item.bad?
  ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator, without return value. `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred.
end

[1, 2, 3].map do |x|
  return if x > 2
  ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator, without return value. `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred.
  x * 2
end

items.select do |item|
  return unless item.valid?
  ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator, without return value. `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred.
end

# Nested blocks: inner argless block, return found by walking to outer iterator
transaction do
  return unless update_necessary?
  items.each do |item|
    return if item.nil?
    ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator, without return value. `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred.
    item.with_lock do
      return if item.stock == 0
      ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator, without return value. `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred.
      item.update!
    end
  end
end

# Return inside iterator inside a method body (was FN before fix)
class Processor
  def process
    @items.each do |item|
      return if item.blank?
      ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator, without return value. `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred.
      item.save!
    end
  end
end

# Return inside iterator inside a class method
class Handler
  def self.run
    TYPES.each do |type, _|
      return if type == :skip
      ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator, without return value. `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred.
    end
  end
end

# Return inside a receiver block of a chained call (and_if_constraint_fails)
TreeNodes::DB_RETRIES.times do
  break if finished

  DB.attempt {
    block.call
    return
    ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator, without return value. `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred.
  }.and_if_constraint_fails {|err|
    last_error = err
  }
end

# Return inside fetch block in receiver of `.each`
def keyspace_changed(keyspace)
  @conditions.fetch(keyspace.name) { return }.each { |c| c.evaluate(keyspace) }
                                     ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator, without return value. `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred.
  nil
end

# Return inside case receiver of a chained map numblock
def parse_hex(hex)
  case hex.length
  when 3 then hex.scan(/./).map { "#{_1}#{_1}" }
  when 6 then hex.scan(/../)
  when 9 then hex.scan(/.../)
  when 12 then hex.scan(/..../)
  else; return
        ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator, without return value. `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred.
  end.map { _1[0, 2].to_i(16) }
end
