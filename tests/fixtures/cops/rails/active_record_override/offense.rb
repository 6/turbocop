def save
    ^^^^ Rails/ActiveRecordOverride: Use `before_save`, `around_save`, or `after_save` callbacks instead of overriding the Active Record method `save`.
  self.title = title.upcase!
  super
end
def destroy
    ^^^^^^^ Rails/ActiveRecordOverride: Use `before_destroy`, `around_destroy`, or `after_destroy` callbacks instead of overriding the Active Record method `destroy`.
  log_deletion
  super
end
def update
    ^^^^^^ Rails/ActiveRecordOverride: Use `before_update`, `around_update`, or `after_update` callbacks instead of overriding the Active Record method `update`.
  normalize_data
  super
end
