User.all.where(active: true)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.order(:name)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.first
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.distinct
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.joins(:articles)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.includes(:articles)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.limit(10)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.ids
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.pick(:id)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.delete_all
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.destroy_all
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
user.articles.all.order(:created_at)
              ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.eager_load(:articles)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.preload(:articles)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.reorder(:created_at)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.left_joins(:articles)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.merge(users)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.find_each(&:do_something)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.take
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.second
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.sole
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.touch_all
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.update_all(name: name)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.calculate(:average, :age)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.delete_by(id: id)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.destroy_by(id: id)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all.find_by!(name: name)
     ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
User.all().order(:created_at)
     ^^^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
class Record < ApplicationRecord
  def self.active
    all.where(active: true)
    ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
  end
end
class Record < ::ApplicationRecord
  def self.recent
    all.order(:created_at)
    ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
  end
end
class Record < ActiveRecord::Base
  def self.names
    all.pluck(:name)
    ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
  end
end
class Record < ::ActiveRecord::Base
  def self.ids_list
    all.ids
    ^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected.
  end
end
