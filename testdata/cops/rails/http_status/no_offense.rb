render status: :ok
render json: data, status: :not_found
head :ok
render plain: "hello"
