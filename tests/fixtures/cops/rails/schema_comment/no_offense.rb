create_table :users, comment: "Stores user accounts" do |t|
  t.string :name
end

create_table :posts, comment: "Blog posts" do |t|
  t.string :title
end
