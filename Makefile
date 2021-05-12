all: test

export SHELL:=bash

DROGUE_NS ?= drogue-iot
CONSOLE_URL ?= $(shell minikube service -n "$(DROGUE_NS)" console --url)
BACKEND_URL ?= $(shell minikube service -n "$(DROGUE_NS)" console-backend --url)
RUST_LOG ?= info

.PHONY: start
start:
	-drg context delete system-tests
	-pkill geckodriver
	$(HOME)/.webdrivers/geckodriver &

.PHONY: stop
stop:
	-pkill geckodriver

.PHONY: test
test: start
	 trap "$(MAKE) stop" EXIT; $(MAKE) test-run

.PHONY: test-run
test-run:
	env \
		CONSOLE_URL=$(CONSOLE_URL) \
		BACKEND_URL=$(BACKEND_URL) \
		RUST_LOG=$(RUST_LOG) \
		TEST_USER=admin \
 		TEST_PASSWORD=admin123456 \
 		TEST_CLIENT_ID=test \
 		TEST_CLIENT_SECRET=$(shell kubectl -n "$(DROGUE_NS)" get secret keycloak-client-secret-test -o "jsonpath={.data.CLIENT_SECRET}" | base64 -d) \
		cargo test -- --test-threads=1 $(TESTS)
