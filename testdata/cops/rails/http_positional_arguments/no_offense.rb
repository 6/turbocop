get :index, params: { user_id: 1 }, headers: { "ACCEPT" => "text/html" }
get :index
post :create, params: { name: "foo" }
