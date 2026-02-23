class CreateUsers < ActiveRecord::Migration[7.0]
  def change
    create_table :users do |t|
      t.string :name
    end
  end
end

class ReversibleExample < ActiveRecord::Migration[7.0]
  def change
    reversible do |dir|
      dir.up do
        execute "ALTER TABLE pages ADD UNIQUE idx (page_id)"
      end
      dir.down do
        execute "ALTER TABLE pages DROP INDEX idx"
      end
    end
  end
end

class UpOnlyExample < ActiveRecord::Migration[7.0]
  def change
    up_only { execute "UPDATE posts SET published = 'true'" }
  end
end

class RemoveWithType < ActiveRecord::Migration[7.0]
  def change
    remove_column(:suppliers, :qualification, :string)
  end
end

class DropWithBlock < ActiveRecord::Migration[7.0]
  def change
    drop_table :users do |t|
      t.string :name
    end
  end
end

class DefaultWithFromTo < ActiveRecord::Migration[7.0]
  def change
    change_column_default(:posts, :state, from: nil, to: "draft")
  end
end
