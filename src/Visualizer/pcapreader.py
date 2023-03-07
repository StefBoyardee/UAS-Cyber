import argparse
from scapy.all import *
from scapy.layers.inet import *
from scapy.layers.l2 import *
import math
import matplotlib.pylab as plt
parser = argparse.ArgumentParser(description="Graphs average packet times per second for each network device.")
parser.add_argument("--file", default="UAV-0-0.pcap", help="A pcap file to read")
args = parser.parse_args()
file_name = args.file

x = []
x.extend(range(1, 200))

cap = rdpcap(file_name)
#first, let's iterate over the packets and grab all used IPs
ips = set()
for pkt in cap:
    if IP in pkt:
        #print(pkt[IP].src)
        ips.add(pkt[IP].src)
print(ips)
#we then iterate over the IPs and for each IP, we do the following:
# - add packets originating from that IP to a list while filtering out unsuccessful packets 
# (right now just checking if it's ICMP, there's almost certainly a better way though)
# once we have this list, we can generate a counter of packets per second
# and feed this into matplotlib!
fig,ax = plt.subplots()
for ip in ips:
    list = []
    for pkt in cap:
        if (IP in pkt) and (pkt[IP].src == ip) and (not ICMP in pkt):
            list.append(pkt)
    times = {} # this is a set of key-value pairs recording how many packets per second exist 
    for pkt in list:
        val = math.floor(pkt.time)
        if val in times: #already recorded packets for this time
            #we instead increment the associated value
            times[val] = times.get(val) + 1
        else:
            times[val] = 1
    #print(times)
    y = []
    for i in x:
        if i in times:
            y.append(times[i])
        else:
            y.append(0)
    print(y)
    ax.plot(x,y,label=ip)
ax.legend();
plt.savefig("graph.png", dpi=500)