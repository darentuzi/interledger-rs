# Simple Two-Node Interledger Payment
> A demo of sending a payment between 2 Interledger.rs nodes without settlement.

## Overview

This example sets up two local Interledger.rs nodes, peers them together, and sends a payment from one to the other. This example does not involve any remote nodes or networks. 

To run the full example, you can use [`run-md.sh`](../../scripts/run-md.sh) as described [here](../README.md). Otherwise, you can walk through each step below.

The logs of each service will be managed by Docker and you can watch the logs by `docker logs -f {container name}`. For instance, run `docker logs -f interledger-rs-node-a` to watch the logs of Node A.

![overview](images/overview-docker.svg)

## Prerequisites

- [Docker](#docker)

### Docker
Because we provide docker images for node and others on [Docker Hub](https://hub.docker.com/u/interledgerrs), in this example we utilize it. You have to install Docker if you don't have already.

- Linux based OSs
    - Ubuntu: [Get Docker Engine - Community for Ubuntu](https://docs.docker.com/install/linux/docker-ce/ubuntu/)
    - Others: [About Docker Engine - Community](https://docs.docker.com/install/)
- macOS: [Install Docker Desktop for Mac](https://docs.docker.com/docker-for-mac/install/)
- Windows: [Install Docker Desktop on Windows](https://docs.docker.com/docker-for-windows/install/)

## Instructions

<!--!
docker --version > /dev/null || printf "\e[31mUh oh! You need to install Docker before running this example\e[m\n"

printf "Stopping Interledger nodes\n"

docker stop redis interledger-rs-node_a interledger-rs-node_b 2> /dev/null
-->

### 0. Clean up Docker
If you have ever tried the other examples, clean up your docker containers because the names may conflict.

```bash #
docker stop `docker ps -aq -f "name=interledger-rs-node"`
docker rm `docker ps -aq -f "name=interledger-rs-node"`

docker stop `docker ps -aq -f "name=interledger-rs-se_"`
docker rm `docker ps -aq -f "name=interledger-rs-se_"`

docker stop `docker ps -aq -f "name=redis"`
docker rm `docker ps -aq -f "name=redis"`
```

### 1. Set up Docker
First, let's set up a docker network so that containers could connect each other. When we start up new containers, we specify this network ID for the new containers.

<!--!
NETWORK_ID=`docker network ls -f "name=interledger" --format="{{.ID}}"`
if [ -z "${NETWORK_ID}" ]; then
    printf "Creating a docker network...\n"
    docker network create interledger
fi
-->
```bash #
docker network create interledger
```

### 2. Launch Redis

<!--!
printf "\nStarting Redis...\n"
-->

```bash
docker start redis ||
docker run --name redis -d -p 6379:6379 --network=interledger redis:5.0.5
```

When you want to watch logs, use the `docker logs` command. You can use the command like: `docker logs redis`.

`--network=interledger` specifies that this container is connected to the network. If there are any other containers connected to the network, they can interact each other. `-p 6379:6379` connects the container's `6379` to your local port `6379` so that we can connect and check the block information.

### 3. Launch 2 Nodes

<!--!

printf "\nStarting nodes...\n"

-->

```bash
# Start both nodes
# Note that the configuration options can be passed as environment variables
# or saved to a YAML file and passed to the node with the `--config` or `-c` CLI argument
# You can also pass it from STDIN

docker start interledger-rs-node_a ||
docker run \
    -e ILP_ADDRESS=example.node_a \
    -e ILP_SECRET_SEED=8852500887504328225458511465394229327394647958135038836332350604 \
    -e ILP_ADMIN_AUTH_TOKEN=admin-a \
    -e ILP_REDIS_CONNECTION=redis://redis:6379/0 \
    -e ILP_HTTP_ADDRESS=0.0.0.0:7770 \
    -e ILP_BTP_ADDRESS=0.0.0.0:7768 \
    -e ILP_SETTLEMENT_ADDRESS=0.0.0.0:7771 \
    -p 7768:7768 \
    -p 7770:7770 \
    -p 7771:7771 \
    --network=interledger \
    --name=interledger-rs-node_a \
    -td \
    interledgerrs/node node

docker start interledger-rs-node_b ||
docker run \
    -e ILP_ADDRESS=example.node_b \
    -e ILP_SECRET_SEED=1604966725982139900555208458637022875563691455429373719368053354 \
    -e ILP_ADMIN_AUTH_TOKEN=admin-b \
    -e ILP_REDIS_CONNECTION=redis://redis:6379/1 \
    -e ILP_HTTP_ADDRESS=0.0.0.0:7770 \
    -e ILP_BTP_ADDRESS=0.0.0.0:7768 \
    -e ILP_SETTLEMENT_ADDRESS=0.0.0.0:7771 \
    -p 8768:7768 \
    -p 8770:7770 \
    -p 8771:7771 \
    --network=interledger \
    --name=interledger-rs-node_b \
    -td \
    interledgerrs/node node
```

<!--!
printf "\nWaiting for Interledger.rs nodes to start up...\n"

function wait_to_serve() {
    while :
    do
        printf "."
        sleep 1
        curl $1 &> /dev/null
        if [ $? -eq 0 ]; then
            break;
        fi
    done
}

wait_to_serve "http://localhost:7770"
wait_to_serve "http://localhost:8770"
printf "\n"

printf "The Interledger.rs nodes are up and running!\n\n"
-->

Now the Interledger.rs nodes are up and running!  
You can also watch the logs with: `docker logs interledger-rs-node_a` or `docker logs interledger-rs-node_b`.

Here are some points that help you understand what we are doing.

- Notice what `-p` options are doing.
    1. Containers have the same local port `7768`, `7770` and `7771`.
    1. On the other hand, the ports are mapped to local ports of `7768`, `7770`, `7771`, `8768`, `8770`, `8771`.
- They are connected to the network `interledger`.
    - So they can communicate each other using host name of `interledger-rs-node_a`, `interledger-rs-node_b` and `redis`.
- `-e ILP_REDIS_CONNECTION` specifies where Redis is located as an environment variable, and the host name is `redis` which will be resolved to the container named `redis`.
- `-e ILP_HTTP_ADDRESS` specifies which port and address the API service will bind its service on as an environment variable, and the address is `0.0.0.0` that means it will be binded to any addresses.
    - `127.0.0.1` wouldn't work because the loopback address is only valid inside the container.

### 4. Configure the Nodes

Let's create accounts on both nodes. The following script sets up accounts for two users, Alice and Bob. It also creates accounts that represent the connection between Nodes A and B.

See the [HTTP API docs](../../docs/api.md) for the full list of fields that can be set on an account.

<!--! printf "Creating accounts:\n\n" -->

```bash
# Create the logs directory if it doesn't already exist
mkdir -p logs

# Insert accounts on Node A
# One account represents Alice and the other represents Node B's account with Node A

printf "Alice's account:\n"
curl \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer admin-a" \
    -d '{
    "ilp_address": "example.node_a.alice",
    "username" : "alice",
    "asset_code": "ABC",
    "asset_scale": 9,
    "max_packet_amount": 100,
    "http_incoming_token": "alice-password"}' \
    http://localhost:7770/accounts

printf "\nNode B's account on Node A:\n"
curl \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer admin-a" \
    -d '{
    "ilp_address": "example.node_b",
    "username" : "node_b",
    "asset_code": "ABC",
    "asset_scale": 9,
    "max_packet_amount": 100,
    "http_incoming_token": "node_b-password",
    "http_outgoing_token": "node_a:node_a-password",
    "http_endpoint": "http://interledger-rs-node_b:7770/ilp",
    "min_balance": -100000,
    "routing_relation": "Peer",
    "send_routes": true,
    "receive_routes": true}' \
    http://localhost:7770/accounts

# Insert accounts on Node B
# One account represents Bob and the other represents Node A's account with Node B

printf "\nBob's Account:\n"
curl \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer admin-b" \
    -d '{
    "ilp_address": "example.node_b.bob",
    "username" : "bob",
    "asset_code": "ABC",
    "asset_scale": 9,
    "max_packet_amount": 100,
    "http_incoming_token": "bob"}' \
    http://localhost:8770/accounts

printf "\nNode A's account on Node B:\n"
curl \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer admin-b" \
    -d '{
    "ilp_address": "example.node_a",
    "username" : "node_a",
    "asset_code": "ABC",
    "asset_scale": 9,
    "max_packet_amount": 100,
    "http_incoming_token": "node_a-password",
    "http_outgoing_token": "node_b:node_b-password",
    "http_endpoint": "http://interledger-rs-node_a:7770/ilp",
    "min_balance": -100000,
    "routing_relation": "Peer",
    "send_routes": true,
    "receive_routes": true}' \
    http://localhost:8770/accounts
```

Note that we send the requests to `localhost:xxxx`. This is because we mapped the local ports to ports of containers, so `localhost` works here.

On the other hand, `http_endpoint` specifies a URL like `http://interledger-rs-node_a:7770/ilp` (`http://{container-name}:{container's port}`). This is because they have to connect to the other connector which runs on the other container, not `localhost`.

<!--! printf "\n\nCreated accounts on both nodes\n\n" -->

### 5. Sending a Payment

<!--!
# check balances before payment
printf "Checking balances...\n"
printf "\nAlice's balance: "
curl \
-H "Authorization: Bearer admin-a" \
http://localhost:7770/accounts/alice/balance

printf "\nNode B's balance on Node A: "
curl \
-H "Authorization: Bearer admin-a" \
http://localhost:7770/accounts/node_b/balance

printf "\nNode A's balance on Node B: "
curl \
-H "Authorization: Bearer admin-b" \
http://localhost:8770/accounts/node_a/balance

printf "\nBob's balance: "
curl \
-H "Authorization: Bearer admin-b" \
http://localhost:8770/accounts/bob/balance

printf "\n\n"
-->

The following script sends a payment from Alice to Bob that is routed from Node A to Node B.

<!--! printf "Sending payment of 500 from Alice (on Node A) to Bob (on Node B)\n" -->

```bash
# Sending payment of 500 from Alice (on Node A) to Bob (on Node B)
curl \
    -H "Authorization: Bearer alice:alice-password" \
    -H "Content-Type: application/json" \
    -d "{\"receiver\":\"http://interledger-rs-node_b:7770/spsp/bob\",\"source_amount\":500}" \
    http://localhost:7770/pay
```

<!--! printf "\n\n" -->

### 6. Check Balances

You can run the following script to print each of the accounts' balances (try doing this before and after sending a payment).

<!--! printf "Checking balances...\n" -->

```bash
printf "\nAlice's balance: "
curl \
-H "Authorization: Bearer admin-a" \
http://localhost:7770/accounts/alice/balance

printf "\nNode B's balance on Node A: "
curl \
-H "Authorization: Bearer admin-a" \
http://localhost:7770/accounts/node_b/balance

printf "\nNode A's balance on Node B: "
curl \
-H "Authorization: Bearer admin-b" \
http://localhost:8770/accounts/node_a/balance

printf "\nBob's balance: "
curl \
-H "Authorization: Bearer admin-b" \
http://localhost:8770/accounts/bob/balance
```

<!--! printf "\n\n" -->

### 7. Clear Redis
If you want to repeat the procedure, you could clear Redis data as follows.

```bash
docker exec redis redis-cli flushall
```

### 8. Kill All the Services
Finally, you can stop all the services as follows:

<!--!
printf "Stopping Interledger nodes\n"
-->

```bash
docker stop redis interledger-rs-node_a interledger-rs-node_b
```

<!--! printf "\n" -->

## Troubleshooting

> Uh oh! You need to install Docker before running this example

You need to install Docker to run this example. See [Prerequisites](#prerequisites) section.

> docker: Error response from daemon: Conflict. The container name "/interledger-rs-node_a" is already in use by container "xxx".
>You have to remove (or rename) that container to be able to reuse that name.

You seem to have run the other example, try [0. Clean up Docker](#0-clean-up-docker) first.

## Conclusion

That's it for this example! You've learned how to set up Interledger.rs nodes, connect them together, and how to send a payment from one to the other.

Check out the [other examples](../README.md) for more complex demos that show other features of Interledger, including settlement, multi-hop routing, and cross-currency payments.
