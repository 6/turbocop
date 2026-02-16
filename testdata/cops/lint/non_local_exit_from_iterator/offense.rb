items.each do |item|
  return item if item > 5
  ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator detected. Use `next` or `break` instead of `return`.
end

[1, 2, 3].map { |x| return x * 2 }
                    ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator detected. Use `next` or `break` instead of `return`.

items.select do |item|
  return true if item.valid?
  ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator detected. Use `next` or `break` instead of `return`.
end
