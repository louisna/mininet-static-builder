# Mininet static builder

This simple program generates all the files necessary to build a Mininet topology with static routing. From a single topology file, it constructs several configuration files that are used for the Mininet topology.

It uses a shortest-path fashion to construct all the paths from all pairs of nodes. A link failure is not recovered as this is static routing.

## Create the topology configuration

```bash
cargo run -- -f <topology file> -d <output directory>
```

This will create three files in the output directory pointed by `-d`. The files are:
* <`topology file name`>-loopbacks,
* <`topology file name`>-links,
* <`topology file name`>-paths,

## Run on Mininet

```
python3 mininet-topo.py --loopbacks <loopbacks file> --links <links file> --path <paths file>
```