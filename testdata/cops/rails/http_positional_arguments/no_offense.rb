get :index, params: { user_id: 1 }, headers: { "ACCEPT" => "text/html" }
get :index
post :create, params: { name: "foo" }
put :update, params: { id: 1, name: "bar" }
delete :destroy, params: { id: 1 }

# Rack::Test::Methods — positional args are correct for Rack tests
class RedirectorTest
  include Rack::Test::Methods
  get "/specs.4.8.gz", {}, "HTTP_HOST" => "example.com"
end
