#imports!
import argparse
from scapy.all import *
from scapy.layers.inet import *
from scapy.layers.l2 import *
import math
import matplotlib.pylab as plt
import os

# program arguments
parser = argparse.ArgumentParser(description="Graphs average packet times per second for each network device.")
parser.add_argument("--file", default="network/UAV-0-0.pcap", help="A pcap file to read")
parser.add_argument("--out", default="output", help="Output directory")
args = parser.parse_args()
file_name = args.file
out_name = args.out + "/"

cap = rdpcap(file_name)
#first, let's iterate over the packets and grab all used IPs
ips = set()
for pkt in cap:
    if IP in pkt:
        ips.add(pkt[IP].src)
print(ips)

#we then walk backwards through the packet capture until we get the last packet 
i = -1
last = cap[i]
while not IP in last:
    i = i - 1
    last = cap[i]
#we now use the last packet time to define our time interval, rounding up
x = []
x.extend(range(1, math.ceil(last.time)))

#we then iterate over the IPs and for each IP, we do the following:
# - add packets originating from that IP to a list while filtering out unsuccessful packets 
# (right now just checking if it's ICMP, there's almost certainly a better way though)
# once we have this list, we can generate a counter of packets per second
# and feed this into matplotlib!

for ip in ips:
    fig,ax = plt.subplots()
    ax.set_title(ip)
    plt.xlabel("Time (s)")
    plt.ylabel("# of Successful outgoing packets")
    for ip2 in ips: #this one keeps track of dest
        if (ip != ip2): #the source to itself will be a flatline so there's no reason to include it
            list = []
            for pkt in cap:
                if (IP in pkt) and (pkt[IP].src == ip) and (pkt[IP].dst == ip2) and (not ICMP in pkt): #packets successfully sent between the specified source ip and destination ip
                    list.append(pkt)
            times = {} # this is a set of key-value pairs recording how many packets per second exist 
            for pkt in list:
                val = math.floor(pkt.time)
                if val in times: #already recorded packets for this time
                    #we instead increment the associated value
                    times[val] = times.get(val) + 1
                else:
                    times[val] = 1
            y = []
            for i in x:
                if i in times:
                    y.append(times[i])
                else:
                    y.append(0)
            ax.plot(x,y,label=ip2)
    ax.legend(title="Destination:");
    os.makedirs(os.path.dirname(out_name), exist_ok=True)
    plt.savefig( out_name + ip + "-graph.png", dpi=500)

