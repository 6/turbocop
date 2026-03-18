render status: :ok
render json: data, status: :not_found
head :ok
render plain: "hello"
redirect_to root_path, status: :moved_permanently
assert_response :ok
assert_redirected_to root_path, status: :moved_permanently
response.head 200
obj.render status: 404
self.redirect_to root_path, status: 301
# Custom status strings with non-standard reason phrases should not be flagged
head "404 AWOL"
head "500 Sorry"
head "599 Whoah!"
