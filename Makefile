has-dir:
	mkdir -p public
build-prod: has-dir
	mkdir -p public/js
	cd frontend && clojure -M:prod
copy-assets: has-dir
	cp frontend/resources/public/*css public
docker-build:
	docker build -t ralexstokes/eth-monitor .
deploy-docker: docker-build
	docker push ralexstokes/eth-monitor
