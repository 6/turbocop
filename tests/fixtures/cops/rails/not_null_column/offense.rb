add_column :users, :name, :string, null: false
                                   ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.

add_column :posts, :title, :string, null: false
                                    ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.

add_column :orders, :status, :integer, null: false
                                       ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.

add_reference :build_configurations, :provider, null: false, foreign_key: true
                                                ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.

class AddUserstampsToCourseUsers < ActiveRecord::Migration[4.2]
  def change
    add_column :course_users,
               :creator_id,
               :integer,
               null: false,
               ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.
               foreign_key: { references: :users }

    add_column :course_users,
               :updater_id,
               :integer,
               null: false,
               ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.
               foreign_key: { references: :users }
  end
end

class AddPackageToProgrammingEvaluation < ActiveRecord::Migration[4.2]
  def change
    change_table :course_assessment_programming_evaluations do |t|
      t.string :package_path, null: false
                              ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.
    end
  end
end

class AddCourseToSkillAndSkillBranch < ActiveRecord::Migration[4.2]
  def change
    change_table :course_assessment_skills do |t|
      t.references :course, null: false
                            ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.
      t.integer :grouping_id, :null => false
                              ^^^^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.
    end

    change_table :course_assessment_skill_branches do |t|
      t.references :course, null: false
                            ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.
    end
  end
end

class ChangeCourseGroupsFromUserToCourseUser < ActiveRecord::Migration[4.2]
  def change
    add_column :course_group_users, :course_user_id, :integer,
               null: false, foreign_key: { references: :course_users }
               ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.
  end
end

add_column :users, :name, :string, null: false, default: nil
                                   ^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.
