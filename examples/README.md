# Interledger.rs Examples

Here you can find various demos of Interledger.rs' functionality:

1. [Simple Two-Node Payment](./simple/README.md)
1. [Two-Node Payment with Ethereum On-Ledger Settlement](./eth-settlement/README.md)
1. [Three-Node Payment with Ethereum and XRP On-Ledger Settlement](./eth_xrp_three_nodes/README.md)
1. Integrating Interledger Into Your App (Coming Soon!)

Have questions? Feel free to [open an issue](https://github.com/emschwartz/interledger-rs/issues/new) or ask a question [on the forum](https://forum.interledger.org/)!

## Running the Examples
The README of each example provides step-by-step instructions on how to run the example.

If you want to run all of the steps automatically, you can use the provided [`run-md.sh`](../scripts/run-md.sh) script to parse and execute the shell commands from the Markdown file:

```bash
# Under the example directory, for example, "simple"
$ ../../scripts/run-md.sh README.md

# It also accepts STDIN:
$ (some command) | ../../scripts/run-md.sh
```

## Examples on Docker
We utilize Docker so that we can provide less dependent, compile-less, thus easy-to-run examples. There are a few things that readers should know about Docker before diving into the examples. If you are already familiar with Docker, you don't need to read this.

For more exact understanding, please read [the official documents](https://docs.docker.com/) of Docker. Here we summarise some of the essences.

1. Docker sets up independent worlds for containers.
1. Docker "images" differ from "containers".

### Docker Sets up Independent Worlds for Containers
Let's imagine a "container" is an independent world built upon your local space's Linux Kernel (technically it could be a more complicated explanation but it should be sufficient for now). It has its own space so some resources can be the same name or id. For instance:

- Container A
    - `/path/to/somewhere`
    - Process A uses port `80`
- Container B
    - `/path/to/somewhere`
    - Process A uses port `80`

Normally this cannot happen because they have the same resources but we can do this on Docker thanks to [`namespaces`](https://docs.docker.com/engine/docker-overview/). These resources can be linked to your local space like:

- Your local space
    - `/container-a` -> Container A's `/path/to/somewhere`
    - `/container-b` -> Container B's `/path/to/somewhere`
    - port `8080` -> Container A's `80`
    - port `8081` -> Container B's `80`

Also, because the containers are independent by default, we need to set up a network if we want to connect these containers each other. Once connected, although it depends on how you connect, Container A can connect to Container B like `container-b:80`, neither `localhost:80` nor `localhost:8081`.

Note that Container B's Process A has to open a port, binding to address `0.0.0.0` or `172.xxx.xxx.xxx` (depends on the situation) not `127.0.0.1`. This is because the loopback address `127.0.0.1` works only inside the container. If Container B's Process A opens a port binding to `127.0.0.1`, Container A cannot connect to the port because Container A's `127.0.0.1` is different from Container B's `127.0.0.1`.

### Docker "images" Differ from "containers"

Imagine that "images" are templates. "Containers" are instantiated from the images, and it could be done repeatedly because they are templates. Even if you remove containers, the images are not affected. Containers could be stopped but it will remain so that we can run them later.

The following is a list of some commands of Docker and what they do. In the examples, we will use these commands so if you are not familiar with Docker, just take a glance.

- `docker run` instantiates containers from images.
- `docker stop` stops containers but the data will remain.
- `docker start` starts stopped containers.
- `docker rm` removes containers but their images will remain.
- `docker rmi` removes images but if there are any containers that are built from the images, we cannot remove.
- `docker ps` lists containers. If you want to include stopped ones, use `docker ps -a`.
- `docker images` lists images.

