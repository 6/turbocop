get ':controller/:action/:id'
get 'photos/:id', to: 'photos#show'
match 'photos/:id', to: 'photos#show', via: [:get, :post]
match 'photos/:id', to: 'photos#show', via: :all
post 'users', to: 'users#create'
patch 'users/:id', to: 'users#update'

# Dynamic hash keys — can't determine route path statically
section = "news"
match section => redirect("/section/#{section}")

# Variable arguments that may contain via: option
match "/app/path", options
match url, opts
