items.each do |item|
  return if item.bad?
  ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator detected. Use `next` or `break` instead of `return`.
end

[1, 2, 3].map do |x|
  return if x > 2
  ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator detected. Use `next` or `break` instead of `return`.
  x * 2
end

items.select do |item|
  return unless item.valid?
  ^^^^^^ Lint/NonLocalExitFromIterator: Non-local exit from iterator detected. Use `next` or `break` instead of `return`.
end
