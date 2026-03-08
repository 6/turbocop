render :show
render template: "users/show"
render plain: "OK"
render json: data
render partial: "form"
renderer.render(inline: "<%= request.ssl? %>")
@view.render(inline: "<%= 'TEXT' %>")
new_renderer.render inline: "test"
