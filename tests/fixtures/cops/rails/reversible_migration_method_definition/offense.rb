class AddNameToUsers < ActiveRecord::Migration[7.0]
^ Rails/ReversibleMigrationMethodDefinition: Migrations must contain either a `change` method, or both an `up` and a `down` method.
  def up
    add_column :users, :name, :string
  end
end

class RemoveOldColumn < ActiveRecord::Migration[7.0]
^ Rails/ReversibleMigrationMethodDefinition: Migrations must contain either a `change` method, or both an `up` and a `down` method.
  def down
    add_column :posts, :legacy, :string
  end
end

class AddIndexToOrders < ::ActiveRecord::Migration[5.2]
^ Rails/ReversibleMigrationMethodDefinition: Migrations must contain either a `change` method, or both an `up` and a `down` method.
  def up
    add_index :orders, :status
  end
end

class ActsAsFollowerMigration < ActiveRecord::Migration[4.2]
^ Rails/ReversibleMigrationMethodDefinition: Migrations must contain either a `change` method, or both an `up` and a `down` method.
  def self.up
    create_table :follows do |t|
      t.references :followable
    end
  end

  def self.down
    drop_table :follows
  end
end

class AddPublisherToSubmissions < ActiveRecord::Migration[4.2]
^ Rails/ReversibleMigrationMethodDefinition: Migrations must contain either a `change` method, or both an `up` and a `down` method.
  change_table :course_assessment_submissions do |t|
    t.integer :publisher_id
    t.datetime :published_at
  end
end
