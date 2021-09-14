# Drogue IoT Cloud – Testing

This project hosts a testing suite for testing a full Drogue IoT Cloud installation end-to-end.

## Running

### Requirements

The following utilities must be installed and available in your PATH :
- Geckodriver : https://github.com/mozilla/geckodriver/relases
- drg : https://github.com/drogue-cloud/drg/releases
- httpie : https://github.com/httpie/httpie/releases

## Run the tests
You will need an installation of Drogue Cloud. If you are using Minikube or Kind, 
you can then run:
    geckodriver
    make CERT_BASE=/path/to/drogue-cloud/build/certs/
Assuming you deployed drogue-cloud from `/path/to/drogue-cloud`.

Otherwise, you need to pass in the application domain:

    make DOMAIN=-my.apps.domain.cluster.tld.

Or explicitly the console and API URL:

    make CONSOLE_URL=https://my-console.tld. API_URL=https://my-api.tld.

## Rust and testing

Rust and testing isn't (yet) as easy as it is with e.g. JUnit. The following sections document some struggles and
how we deal with them.

### Setup and teardown

Setup and teardown of resources, especially `async` ones (like the web driver) are not supported by the default Rust
test runner.

For this, we use `test-context`. We can setup and tear down the resources, driven by the `test-context` crate, which
also supports `async_trait` and so we can easily deconstruct the web driver.

### Parametrized tests

Some tests require to be run with different parameter sets. Like the MQTT integration, we can run this with MQTT 3.1.1
and 5.0 in both structured and binary mode.

Instead of defining each test three times, we use the `rstest` crate, which allows to define something like a test
matrix. We don't use `rstest-reuse` right now, as it has some limitations, which might get resolved in the near future.

Unfortunately, `rstest` does not support custom/async teardown of resources. And so, we need to rely on the `Drop`
trait and do some blocking/async magic in there, do properly destruct the web driver. This requires `rstest` based
tests to be run with the "multi-threaded" Tokio runtime. Which needs to be explicitly enabled with tests.

Hopefully, this will be fixed too in `rstest`, which would allow us to switch from `test-context` to `rstest` and
drop the `Drop` workaround.
