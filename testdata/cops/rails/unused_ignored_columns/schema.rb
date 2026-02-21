ActiveRecord::Schema[7.0].define(version: 2024_01_01) do
  create_table "users", force: :cascade do |t|
    t.string "account"
    t.string "name"
    t.string "email"
    t.integer "role"
    t.string "status"
  end
end
