rand(1..6)
rand(6)
rand(1...7)
foo + 1
rand
1 + 2

# rand with variable offset is not flagged (offset must be integer literal)
rand(6) + offset
variable + rand(6)
rand(6) + some_method
