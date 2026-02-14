ActiveRecord::Base.transaction do
  raise ActiveRecord::Rollback if user.nil?
  user.save!
end
