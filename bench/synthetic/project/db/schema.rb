# This file is auto-generated from the current state of the database.
ActiveRecord::Schema[7.0].define(version: 2025_01_01_000003) do
  create_table "users", force: :cascade do |t|
    t.string "name"
    t.string "email"
    t.boolean "active"
    t.string "role"
    t.integer "department_id"
    t.timestamps
  end

  create_table "posts", force: :cascade do |t|
    t.string "title"
    t.text "body"
    t.boolean "published"
    t.boolean "featured"
    t.integer "user_id"
    t.timestamps
  end

  create_table "records", force: :cascade do |t|
    t.string "save"
    t.string "class"
    t.timestamps
  end
end
