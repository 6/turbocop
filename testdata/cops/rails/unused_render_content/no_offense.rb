render status: :no_content
render json: { data: "ok" }
render status: :ok, json: { data: "ok" }
head :no_content