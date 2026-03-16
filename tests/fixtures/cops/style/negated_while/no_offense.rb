until x
  do_something
end

while x
  do_something
end

while x > 0
  process
end

until done?
  work
end

while !!flag
  do_something
end

while obj&.empty?&.!
  do_something
end

x += 1 while condition

# begin...end while/until loops (do-while) — RuboCop does not flag these
begin
  password = ask("Password: ")
  confirmed = password == expected
end while !confirmed

begin
  input = gets.chomp
end while !%w(y n).include?(input)

begin
  result = try_operation
end until !result.success?
