status == "active"
status.active?
role == "admin"
name.present?
Rails.env.production?
# inquiry on a method call result (not a literal) is not flagged
ROLES.key(flag)&.inquiry
some_method.inquiry
variable.inquiry
