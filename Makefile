all: test

export SHELL:=bash

DROGUE_NS ?= drogue-iot

CLUSTER ?= minikube

PROTO ?= http

ifeq ($(CLUSTER), minikube)
DOMAIN=$(shell minikube ip).nip.io
else ifeq ($(CLUSTER), kind)
DOMAIN=$(shell 'kubectl get node kind-control-plane -o jsonpath='"'"'{.status.addresses[?(@.type == "InternalIP")].address}'"'").nip.io
else
$(error Unknown cluster type: $(CLUSTER))
endif

CONSOLE_URL ?= $(PROTO)://console.$(DOMAIN)
API_URL ?= $(PROTO)://api.$(DOMAIN)
RUST_LOG ?= info

.PHONY: info
info:
	@echo API: $(API_URL)
	@echo Console: $(CONSOLE_URL)
	@echo NS: $(DROGUE_NS)

.PHONY: start
start: info
	-drg context delete system-tests
	-pkill geckodriver
	geckodriver &

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
		API_URL=$(API_URL) \
		RUST_LOG=$(RUST_LOG) \
		TEST_USER=admin \
 		TEST_PASSWORD=admin123456 \
 		TEST_CLIENT_ID=test \
 		TEST_CLIENT_SECRET=$(shell kubectl -n "$(DROGUE_NS)" get secret keycloak-client-secret-test -o "jsonpath={.data.CLIENT_SECRET}" | base64 -d) \
		cargo test -- --test-threads=1 $(TESTS)
