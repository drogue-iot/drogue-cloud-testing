all: info test

export SHELL:=bash

DROGUE_NS ?= drogue-iot

CLUSTER ?= minikube

PROTO ?= http

ifeq ($(CLUSTER), minikube)
DOMAIN=$(shell minikube ip).nip.io
else ifeq ($(CLUSTER), kind)
DOMAIN=$(shell kubectl get node kind-control-plane -o jsonpath='{.status.addresses[?(@.type == "InternalIP")].address}').nip.io
else
$(error Unknown cluster type: $(CLUSTER))
endif

CONSOLE_URL ?= $(PROTO)://console.$(DOMAIN)
API_URL ?= $(PROTO)://api.$(DOMAIN)
RUST_LOG ?= info

ifndef CERT_BASE
$(error Missing CERT_BASE variable. This is typically 'build/certs' in the directory you ran the installation)
endif

.PHONY: info
info:
	@echo API: $(API_URL)
	@echo Console: $(CONSOLE_URL)
	@echo NS: $(DROGUE_NS)
	@echo Certificate base: $(CERT_BASE)

.PHONY: start
start:
	-drg context delete system-tests
	-pkill geckodriver
	geckodriver &>/dev/null &

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
 		CERT_BASE=$(CERT_BASE) \
		cargo test -- --test-threads=1 $(TEST_ARGS) $(TESTS)
