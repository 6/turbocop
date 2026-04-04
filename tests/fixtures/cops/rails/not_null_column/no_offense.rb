add_column :users, :name, :string, null: false, default: ""
add_column :users, :name, :string
add_column :users, :age, :integer, null: true
add_column :posts, :title, :string, null: false, default: ""
add_column :users, :email, :string
add_reference :products, :category, null: false, default: 1
add_column :daily_query_noresults_stats, :locale, :null => false
add_column :users, :height_in, :virtual, as: "height_cm / 2.54", null: false, default: nil

change_table :users do |t|
  t.string :name, null: false, default: ""
  t.references :address
end

change_table :test_group_results do |t|
  t.change :created_at, :timestamp, null: false
end

class DropLocaleFromDailyQueryNoresultsStats < ActiveRecord::Migration
  def self.up
    remove_column :daily_query_noresults_stats, :locale
  end

  def self.down
    add_column :daily_query_noresults_stats, :locale, :null => false
  end
end
