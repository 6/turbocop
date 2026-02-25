3.times { create :user }
^^^^^^^ FactoryBot/CreateList: Prefer create_list.
3.times.map { create :user }
^^^^^^^^^^^ FactoryBot/CreateList: Prefer create_list.
5.times { create(:user, :trait) }
^^^^^^^ FactoryBot/CreateList: Prefer create_list.
[create(:user), create(:user)]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ FactoryBot/CreateList: Prefer create_list.
[create(:user, :admin), create(:user, :admin), create(:user, :admin)]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ FactoryBot/CreateList: Prefer create_list.
[create(:user, point: rand), create(:user, point: rand)]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ FactoryBot/CreateList: Prefer 2.times.map.
