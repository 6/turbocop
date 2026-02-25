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
