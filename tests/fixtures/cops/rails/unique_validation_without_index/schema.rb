ActiveRecord::Schema[7.0].define(version: 2024_01_01) do
  create_table "users", force: :cascade do |t|
    t.string "account"
    t.string "email"
    t.string "username"
    t.string "name"
    t.bigint "organization_id"
    t.index ["email"], unique: true
  end
end
