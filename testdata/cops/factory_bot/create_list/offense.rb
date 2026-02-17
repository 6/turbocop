3.times { create :user }
^^^^^^^ FactoryBot/CreateList: Prefer create_list.
3.times.map { create :user }
^^^^^^^^^^^ FactoryBot/CreateList: Prefer create_list.
5.times { create(:user, :trait) }
^^^^^^^ FactoryBot/CreateList: Prefer create_list.
