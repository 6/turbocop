get '/users', '/other_path', to: 'users#index'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/MultipleRoutePaths: Use separate routes instead of combining multiple route paths in a single route.
post '/a', '/b', to: 'c#d'
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/MultipleRoutePaths: Use separate routes instead of combining multiple route paths in a single route.
put '/x', '/y', '/z', to: 'w#v'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/MultipleRoutePaths: Use separate routes instead of combining multiple route paths in a single route.
