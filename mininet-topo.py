from mininet.topo import Topo
from mininet.net import Mininet
from mininet.util import dumpNodeConnections
from mininet.cli import CLI
from mininet.node import OVSController

import argparse
import os
import shutil


PCAP_DIR = "pcaps"


class MyAutoTopo(Topo):
    def build(self, **kwargs):
        with open(kwargs["loopbacks"]) as fd:
            txt = fd.read().split("\n")
        all_nodes = dict()
        for node_info in txt:
            if len(node_info) == 0:
                continue
            node_id, loopback = node_info.split(" ")
            node = self.addHost(node_id, ip=loopback,
                                mac=f"00:00:00:00:00:{node_id}")
            all_nodes[node_id] = node

        all_links = set()
        with open(kwargs["links"]) as fd:
            txt = fd.read().split("\n")
        for link_info in txt:
            if len(link_info) == 0:
                continue
            id1, id2, _, _, _ = link_info.split(" ")
            if (id1, id2) in all_links or (id2, id1) in all_links:
                continue
            self.addLink(all_nodes[id1], all_nodes[id2])
            all_links.add((id1, id2))


def simpleRun(args):
    ipv4 = args.ipv4
    print("USE IPV4", ipv4)

    args_dict = {
        "loopbacks": args.loopbacks,
        "links": args.links,
        "paths": args.paths
    }
    topo = MyAutoTopo(**args_dict)
    net = Mininet(topo=topo, controller=OVSController)
    net.start()

    dumpNodeConnections(net.hosts)

    nb_nodes = 0
    loopbacks = dict()

    with open(args_dict["loopbacks"]) as fd:
        txt = fd.read().split("\n")
        for loopback_info in txt:
            if len(loopback_info) == 0:
                continue
            id1, loopback = loopback_info.split(" ")
            if ipv4:
                cmd = f"ip addr add {loopback} dev lo"
            else:
                cmd = f"ip -6 addr add {loopback} dev lo"
            loopbacks[id1] = loopback[:-3]
            print(cmd)
            net[id1].cmd(cmd)
            # Enable IPv6 forwarding
            cmd = "sysctl net.ipv4.ip_forward=1"
            net[id1].cmd(cmd)
            cmd = "sysctl net.ipv6.conf.all.forwarding=1"
            net[id1].cmd(cmd)
            cmd = "sysctl net.ipv6.conf.all.mc_forwarding=1"
            net[id1].cmd(cmd)
            cmd = "sysctl net.ipv6.conf.lo.forwarding=1"
            net[id1].cmd(cmd)
            cmd = "sysctl net.ipv6.conf.lo.mc_forwarding=1"
            net[id1].cmd(cmd)
            nb_nodes += 1

    with open(args_dict["links"]) as fd:
        txt = fd.read().split("\n")
        for link_info in txt:
            if len(link_info) == 0:
                continue
            id1, _, itf, link, loopback = link_info.split(" ")
            cmd = f"sysctl net.ipv6.conf.{id1}-eth{itf}.forwarding=1"
            net[id1].cmd(cmd)
            cmd = f"sysctl net.ipv6.conf.{id1}-eth{itf}.mc_forwarding=1"
            net[id1].cmd(cmd)

            cmd = f"sysctl net.ipv4.{id1}-eth{itf}.ip_forward=1"
            net[id1].cmd(cmd)
            if ipv4:
                cmd = f"ip addr add {link} dev {id1}-eth{itf}"
            else:
                cmd = f"ip -6 addr add {link} dev {id1}-eth{itf}"
            print(id1, cmd)
            net[id1].cmd(cmd)

            # Start a tcpdump capture for each interface
            # From https://stackoverflow.com/questions/43765117/how-to-check-existence-of-a-folder-and-then-remove-it
            if args.traces:
                if os.path.exists(PCAP_DIR) and os.path.isdir(PCAP_DIR):
                    shutil.rmtree(PCAP_DIR)
                os.makedirs(PCAP_DIR)
                net[id1].cmd(f"tcpdump -i {id1}-eth{itf} -w {id1}-{itf}.pcap &")

    with open(args_dict["paths"]) as fd:
        txt = fd.read().split("\n")
        for path_info in txt:
            if len(path_info) == 0:
                continue
            id1, itf, link, loopback = path_info.split(" ")
            if ipv4:
                cmd = f"ip route add {loopback} via {link}"
            else:
                cmd = f"ip -6 route add {loopback} via {link}"
            print(id1, cmd)
            net[id1].cmd(cmd)

    # Make your own emulation here
    # ...

    CLI(net)
    net.stop()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("-l", "--loopbacks", type=str,
                        default="configs/topo-loopbacks.txt")
    parser.add_argument("-i", "--links", type=str, default="configs/topo-links.txt")
    parser.add_argument("-p", "--paths", type=str, default="configs/topo-paths.txt")
    parser.add_argument("-t", "--traces", action="store_true", help="Activate tracing with tcpdump")
    parser.add_argument("--ipv4", action="store_true", help="Indicates that the scripts use IPv4 instead of IPv6")
    args = parser.parse_args() 
    simpleRun(args)
