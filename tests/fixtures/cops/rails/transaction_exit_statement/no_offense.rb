ActiveRecord::Base.transaction do
  raise ActiveRecord::Rollback if user.nil?
  user.save!
end

ActiveRecord::Base.transaction do
  User.create!(name: "test")
end
