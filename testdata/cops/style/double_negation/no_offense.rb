!something

x = !foo

y = true

z = false

result = condition ? true : false

# allowed_in_returns (default): !! at end of method body is OK
def active?
  !!@active
end

def valid?
  !!validate
end

def admin?
  !!current_user&.admin
end

# !! as part of a larger expression in return position
def comparison?
  !!simple_comparison(node) || nested_comparison?(node)
end

def allow_if_method_has_argument?(send_node)
  !!cop_config.fetch('AllowMethodsWithArguments', false) && send_node.arguments.any?
end
