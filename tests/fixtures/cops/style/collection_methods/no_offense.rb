[1, 2, 3].map { |e| e + 1 }
[1, 2, 3].reduce { |a, e| a + e }
[1, 2, 3].find { |e| e > 1 }
[1, 2, 3].select { |e| e > 1 }
[1, 2, 3].include?(1)
x = [1, 2, 3]

# Plain method calls without blocks should NOT be flagged
# (RuboCop only flags when there's a block or block_pass)
modified.member? guess
org.member?(@cust)
plan.entitlements.member?(entitlement.to_s)
%w[i l o 1 0].member?(v)
[1, 2, 3].collect
[1, 2, 3].inject
[1, 2, 3].detect
[1, 2, 3].find_all

# Methods with regular args (not block_pass) should NOT be flagged
[1, 2, 3].collect(:+)
[1, 2, 3]&.collect(:+)
