before { create_users }

after { cleanup_users }

around { |example| example.run }

before(:all) do
  create_users
  create_products
end
