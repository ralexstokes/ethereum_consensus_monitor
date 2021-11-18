# ethereum consensus monitor

Software to monitor ethereum's consensus layer.

# how to build

## build the frontend

`cd frontend && clojure -M:prod`

this will build a production version of the frontend and install it to the backend's
web assets under `public`.

if any of the static web assets change from their source in the frontend,
they can be copied to the correct location for the backend with

`just copy-assets`

## build a docker image

to build a self-contained docker image:

`just docker-build`

and/or deploy with:

`just deploy-docker`

# how to run

Everything the monitor needs to run is given in the config.

An example configuration file is provided in `config.example.toml`.

To run e.g. via `cargo`:

`cargo run -- --config-path config.example.toml`

## development

to run a development server for the frontend:

`cd frontend && clj -M:dev`

this also needs the backend running locally

# API documentation

the following routes are exposed under `/api/v1`:

- /network-config
  - return data relevant to the connected network
- /nodes
  - return status of the nodes under monitoring
- /chain
  - return status of the beacon chain
- /fork-choice
  - return data for the fork-choice visualization
- /participation
  - return data for attestation and sync committee participation
- /deposit-contract
  - return data about the deposit contract
- /weak-subjectivity
  - return data about weak subjectivity in the network

# TODO

- match v1 functionality

- fetch head for prysm or nimbus
- test syncing status

- get a "participation provider"
- another pass at attestation participation

- update stake percents on fork choice
- other features
  - deposit contract monitor
  - wsprovider

-- send as updates over ws?
- /spec
- /nodes
- /fork-choice
- /participation
- /deposit-contract
- /ws-data
