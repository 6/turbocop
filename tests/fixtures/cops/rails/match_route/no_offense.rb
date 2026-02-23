get ':controller/:action/:id'
get 'photos/:id', to: 'photos#show'
match 'photos/:id', to: 'photos#show', via: [:get, :post]
match 'photos/:id', to: 'photos#show', via: :all
post 'users', to: 'users#create'
patch 'users/:id', to: 'users#update'
