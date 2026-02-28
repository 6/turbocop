x.start_with?("a", "b")
x.end_with?(".rb", ".py")
x.start_with?("a") || x.end_with?("b")
x.start_with?("a") && x.start_with?("b")
x.include?("a") || x.include?("b")
# Different receivers should not flag
x.start_with?("a") || y.start_with?("b")
controller_name.start_with?(":") || action_name.start_with?(":")
# Interpolated string arguments are impure
href.start_with?("#{base_path}/c/") || href.start_with?("#{base_path}/tag/")
# Method call arguments are impure
url.start_with?(base_url) || url.start_with?(cdn_url)
path.start_with?(rule) || path.start_with?('/' + rule)
# Negated with only one side negated — not a valid pattern
!x.start_with?("a") || x.start_with?("b")
x.start_with?("a") || !x.start_with?("b")
!x.end_with?("a") || x.end_with?("b")
# Non-negated && should not flag
x.start_with?("a") && x.start_with?("b")
# Different method names
!x.start_with?("a") && !x.end_with?("b")
