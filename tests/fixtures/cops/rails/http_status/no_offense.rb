render status: :ok
render json: data, status: :not_found
head :ok
render plain: "hello"
redirect_to root_path, status: :moved_permanently
assert_response :ok
assert_redirected_to root_path, status: :moved_permanently
