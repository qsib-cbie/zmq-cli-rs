### ZMQ CLI in Rust

This repo provides implementations from https://github.com/erickt/rust-zmq/tree/master/examples and http://zguide.zeromq.org behind a command line interface with basic logging.

#### Building on Ubuntu 20.04

Some examples have been run on macOS or Ubuntu 20.04 (cheapest Digital Ocean droplet)

Required
```
apt install gcc build-essential libzmq3-dev pkg-config
```

Required plus nice to haves
```
apt install gcc build-essential libzmq3-dev pkg-config glances

```

#### Streaming a file (like a video through VLC)

Start up order does not matter since nothing runs until everything is connected

Start the proxy at a well known ip
```
cargo run --release -- -vv start --routine streamfile -1 tcp://0.0.0.0:5555 -2 tcp://0.0.0.0:5556 --socket-type proxy
```

Start the data generator
```
mkfifo stream.h264
raspivid -ih -n -t 0 -o stream.h264
```

Start the stream server
```
cargo run --release -- -v start --routine streamfile -1 tcp://<well known ip address>:5555 --socket-type server
```

Start the N stream client
```
mkfifo stream.h264
cargo run --release -- -v start --routine streamfile -1 tcp://<well known ip address>:5556 --socket-type client
```

Consume stream data
```
vlc stream.h264 :demux=h264
```