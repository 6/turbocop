before_save :upcase_title
after_destroy :log_deletion
before_update :normalize_data
def custom_method
  super
end
def save
  # No super call
  do_something
end
