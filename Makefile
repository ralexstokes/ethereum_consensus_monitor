has-dir:
	mkdir -p public
build-prod: has-dir
	mkdir -p public/js
	cd frontend && clojure -M:prod
copy-assets: has-dir
	cp frontend/resources/public/*css public
build-docker:
	docker build -t ralexstokes/ethereum-consensus-monitor .
push-docker:
	docker push ralexstokes/ethereum-consensus-monitor
deploy-docker: build-docker push-docker
