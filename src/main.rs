#[macro_use]
extern crate log;

use clap::Parser;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as fmtWrite;
use std::fs::File;
use std::hash::Hash;
use std::io::Write;
use std::io::{BufRead, BufReader};
mod dijkstra;
use dijkstra::dijkstra;

type McAddr = Vec<(usize, String)>;

#[derive(Debug)]
enum Error {
    /// Impossible to parse the file to crate a topo.
    FileParse,

    /// Missing node.
    MissingNone,

    /// Too many multicast addresses.
    TooManyMcAddrs,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Parser)]
struct Args {
    /// Topology NTF-like file.
    #[clap(value_parser)]
    topo_file: String,

    /// Path containing the output files.
    #[clap(short = 'd', long = "directory", value_parser)]
    directory: String,

    /// Use IPv4 instead of IPv6.
    #[clap(long = "ipv4")]
    ipv4: bool,

    /// Add a multicast path from the specified source.
    /// Is assumed to follow the `ipv4` flag.
    /// Currently only supports a single multicast address.
    #[clap(short = 'm', long = "multicast", value_parser)]
    multicast: Option<String>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Node {
    id: usize,
    name: String,
    neighbours: Vec<(usize, i32)>, // (id, cost)
}

struct Graph {
    nodes: Vec<Node>,
    node2id: HashMap<String, usize>,
}

impl Graph {
    fn from_file(file_path: &str) -> Result<Self> {
        let file = std::fs::File::open(file_path).map_err(|_| Error::FileParse)?;

        let mut nodes = Vec::new(); // We do not know the size at first.
        let reader = BufReader::new(file);
        let mut node2id = HashMap::new();
        let mut current_id = 0;

        for line in reader.lines() {
            let line = line.unwrap();
            let split: Vec<&str> = line.split(' ').collect();
            let a_id: usize = *node2id.entry(split[0].to_string()).or_insert(current_id);
            if a_id == current_id {
                current_id += 1;
                let node = Node {
                    name: split[0].to_string(),
                    neighbours: Vec::new(),
                    id: a_id,
                };
                nodes.push(node);
            }

            let b_id: usize = *node2id.entry(split[1].to_string()).or_insert(current_id);
            if b_id == current_id {
                current_id += 1;
                let node = Node {
                    name: split[1].to_string(),
                    neighbours: Vec::new(),
                    id: b_id,
                };
                nodes.push(node);
            }

            // Get the metric from the line
            let metric: i32 = split[2].parse::<i32>().unwrap();

            // Add in neighbours adjacency list
            nodes[a_id].neighbours.push((b_id, metric));
            nodes[b_id].neighbours.push((a_id, metric));
        }

        Ok(Graph { nodes, node2id })
    }

    fn get_neighbours(&self) -> Vec<Vec<(usize, i32)>> {
        self.nodes
            .iter()
            .map(|node| node.neighbours.to_owned())
            .collect()
    }

    fn get_mininet_config(
        &self,
        directory: &str,
        file_prefix: &str,
        ipv4: bool,
        mc_addrs: Option<McAddr>,
    ) -> Result<()> {
        let nb_nodes = self.nodes.len();
        let topo = &self.nodes;
        let successors = self.get_neighbours();

        // Set the loopbacks.
        let mut loopbacks = HashMap::new();
        let mut s = String::new();
        for i in 0..nb_nodes {
            let lo = if ipv4 {
                format!("11.0.{}.1/32", i)
            } else {
                format!("babe:cafe:{:x}::1/64", i)
            };
            writeln!(s, "{} {}", i, lo).unwrap();
            loopbacks.insert(i, lo);
        }
        let pathname = format!("{}-loopbacks.txt", file_prefix);
        let path = std::path::Path::new(directory).join(pathname);
        let mut file = File::create(&path).unwrap();
        file.write_all(s.as_bytes()).unwrap();

        let prefix_length = if ipv4 { 30 } else { 64 };

        // Set the links.
        let mut s = String::new();
        let base_link = if ipv4 { "11.1" } else { "babe:cafe:dead" };
        let mut links = HashMap::new();
        for i in 0..nb_nodes {
            let node_a = &topo[i];
            let id_a = node_a.id;
            for (jj, (j, _)) in node_a.neighbours.iter().enumerate() {
                let node_b = &topo[*j];
                let id_b = node_b.id;
                if links.contains_key(&(i, *j)) || links.contains_key(&(*j, i)) {
                    // The link exists but not the command
                    let link_a_b = links
                        .get(&(*j, i))
                        .ok_or_else(|| links.get(&(i, *j)))
                        .unwrap();
                    writeln!(
                        s,
                        "{} {} {} {}/{} {}",
                        id_a,
                        *j,
                        jj,
                        link_a_b,
                        prefix_length,
                        loopbacks.get(&(id_b as usize)).unwrap()
                    )
                    .unwrap();
                    continue;
                }
                let link_a_b = if ipv4 {
                    format!("{}.{}.1", base_link, id_a * nb_nodes + id_b)
                } else {
                    format!("{}:{:x}{:x}::1", base_link, id_a, id_b)
                };
                let link_b_a = if ipv4 {
                    format!("{}.{}.2", base_link, id_a * nb_nodes + id_b)
                } else {
                    format!("{}:{:x}{:x}::2", base_link, id_a, id_b)
                };

                writeln!(
                    s,
                    "{} {} {} {}/{} {}",
                    id_a,
                    *j,
                    jj,
                    link_a_b,
                    prefix_length,
                    loopbacks.get(&(id_b as usize)).unwrap()
                )
                .unwrap();

                links.insert((i, *j), link_b_a);
                links.insert((*j, i), link_a_b);
            }
        }
        let pathname = format!("{}-links.txt", file_prefix);
        let path = std::path::Path::new(directory).join(pathname);
        let mut file = File::create(&path).unwrap();
        file.write_all(s.as_bytes()).unwrap();

        // Finally all the paths must be statically added for each router.
        let prefix_length = if ipv4 { 32 } else { 64 };
        let mut s = String::new();
        for source in 0..nb_nodes {
            let predecessors = dijkstra(&successors, &source).unwrap();
            debug!("PREDECESSORS: {:?}", predecessors);

            // Construct the next hop mapping, possibly there are multiple paths so multiple output interfaces.
            let next_hop: Vec<Vec<usize>> = (0..nb_nodes)
                .map(|i| get_all_out_interfaces_to_destination(&predecessors, source, i))
                .collect();
            debug!("MAPPING: {:?}", next_hop);
            let node = topo.get(source).unwrap();

            // For each destination, find the correct next hop.
            for (i, dst) in next_hop.into_iter().enumerate() {
                if i == source {
                    continue; // Same node.
                }
                // Only use the first path.
                // `hop` is the node id of the next hop
                let hop = dst[0];

                let link_ip = links.get(&(source, hop)).unwrap();
                let destination_ip = loopbacks.get(&i).unwrap();

                // Get the output interface of the node.
                let output_itf = node.neighbours.iter().position(|&(r, _)| r == hop).unwrap();

                // Hop is not correct here!
                writeln!(
                    s,
                    "{} {} {} {}",
                    source, output_itf, link_ip, destination_ip
                )
                .unwrap();

                // Add the same path for each link local of the destination node.
                for (&(_, dst), link) in links.iter().filter(|(&(src, _), _)| src == i) {
                    if dst == source {
                        continue;
                    }
                    writeln!(
                        s,
                        "{} {} {} {}/{}",
                        source, output_itf, link_ip, link, prefix_length
                    )
                    .unwrap();
                }
            }
        }

        let pathname = format!("{}-paths.txt", file_prefix);
        let path = std::path::Path::new(directory).join(pathname);
        let mut file = File::create(&path).unwrap();
        file.write_all(s.as_bytes()).unwrap();

        // Multicast routes to be installed.
        if let Some(mc_addrs) = mc_addrs.as_ref() {
            let mut s = String::new();
            for (source, mc_addr) in mc_addrs {
                let predecessors = dijkstra(&successors, source).unwrap();
                println!("predecessors is {:?}", predecessors);
                for node_id in 0..nb_nodes {
                    if node_id == *source {
                        continue;
                    }
                    let node = topo.get(node_id).unwrap();
                    let in_itf = node.neighbours.iter().position(|(r, _)| r == predecessors.get(&node_id).unwrap()[0]).unwrap();
                    let successors = get_successors(&predecessors, node_id);
                    let intfs = successors.iter().map(|succ| node.neighbours.iter().position(|(r, _)| r == succ).unwrap()).fold(String::new(), |string, itf| format!("{} {}", string, itf).to_string());
                    if !successors.is_empty() {
                        writeln!(s,
                        "{} {} {}/{} {}",
                        node_id, in_itf, mc_addr, prefix_length, intfs,
                    ).unwrap()
                    }
                }
            }
            let pathname = format!("{}-multicast-paths.txt", file_prefix);
            let path = std::path::Path::new(directory).join(pathname);
            let mut file = File::create(&path).unwrap();
            file.write_all(s.as_bytes()).unwrap();
        }

        Ok(())
    }
}

fn get_all_out_interfaces_to_destination(
    predecessors: &HashMap<&usize, Vec<&usize>>,
    source: usize,
    destination: usize,
) -> Vec<usize> {
    if source == destination {
        return vec![source];
    }

    let mut out: Vec<usize> = Vec::new();
    let mut visited = vec![false; predecessors.len()];
    let mut stack = VecDeque::new();
    stack.push_back(destination);
    while !stack.is_empty() {
        let elem = stack.pop_back().unwrap();
        if visited[elem] {
            continue;
        }
        visited[elem] = true;
        for &&pred in predecessors.get(&elem).unwrap() {
            if pred == source {
                out.push(elem);
                continue;
            }
            if visited[pred] {
                continue;
            }
            stack.push_back(pred);
        }
    }
    out
}

fn get_mc_addrs(filename: &str, graph: &Graph) -> Result<McAddr> {
    let mut out = McAddr::new();

    let file = std::fs::File::open(filename).map_err(|_| Error::FileParse)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.unwrap();
        let split: Vec<_> = line.split(' ').collect();
        let id: usize = *graph.node2id.get(split[0]).ok_or(Error::MissingNone)?;
        out.push((id, split[1].to_string()));
    }

    if out.len() > 1 {
        return Err(Error::TooManyMcAddrs);
    }

    Ok(out)
}

fn get_successors(predecessors: &HashMap<&usize, Vec<&usize>>, node: usize) -> HashSet<usize> {
    predecessors
        .iter()
        .filter(|(_, preds)| preds.contains(&&node))
        .map(|(k, _)| **k)
        .collect()
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    let graph = Graph::from_file(&args.topo_file).unwrap();
    let mc_addrs = if let Some(file) = args.multicast.as_ref() {
        Some(get_mc_addrs(file, &graph).unwrap())
    } else {
        None
    };
    let path = std::path::Path::new(&args.topo_file);
    let filename = path.file_stem().unwrap().to_str().unwrap();
    graph
        .get_mininet_config(&args.directory, filename, args.ipv4, mc_addrs)
        .unwrap();
}
