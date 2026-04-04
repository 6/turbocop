ActiveRecord::Schema[7.0].define(version: 2024_01_01) do
  create_table "users", force: :cascade do |t|
    t.string "account"
    t.string "name"
    t.string "email"
    t.integer "role"
    t.string "status"
  end

  create_table "editions", force: :cascade do |t|
    t.string "title"
  end

  create_table "offsite_links", force: :cascade do |t|
    t.bigint "parent_id"
  end

  create_table "accounts", force: :cascade do |t|
    t.string "username"
  end
end
