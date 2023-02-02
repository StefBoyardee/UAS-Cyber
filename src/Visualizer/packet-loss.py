import argparse
import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np

def parse_csv_line(line):
    parts = line.split(",")
    if parts[0] != "color":
        result = {}
        result["time"] = float(parts[0])
        result["ip"] = parts[1]
        result["x"] = float(parts[2])
        result["y"] = float(parts[3])
        result["z"] = float(parts[4])
        return result


parser = argparse.ArgumentParser(description="Graphs average packet times for each network device. This borrows a lot of code from vis.py!")
parser.add_argument("--file", default="positions.csv", help="A positions CSV file to read")
args = parser.parse_args()
file_name = args.file

print("Opening input CSV file: " + file_name)
csv_file = open(file_name, "r")
raw_lines = csv_file.read().splitlines()
csv_file.close()
csv_header = raw_lines[0]
lines = []
devices = []
#the source node isnt listed in the same format as the other devices
#we have to append it manually
devices.append(csv_header.split(",")[5])

for line in raw_lines[1:]:
    #Parse raw lines and stick into lines
    parsed = parse_csv_line(line)
    if (parsed != None):
        if (parsed.get('time') == 0): #register network devices while were at it
            devices.append(parsed.get('ip'))
        lines.append(parsed)
    
#print(lines)
print(devices)
#register plot
fig,ax = plt.subplots()


for device in devices:
    sorted = []
    for line in lines:
        if line.get('ip') == device:
            sorted.append(line)

    #print(sorted)
    #now we have all the relevant data to just this device
    curr_time = 0
    prev_time = 0
    cnt = 0
    x = []
    y = []
    for entry in sorted:
        cnt+=1
        curr_time = entry.get('time')
        if (curr_time - prev_time) >= 1: #we're counting packets per second
            x.append(curr_time)
            y.append(cnt/(curr_time - prev_time))
            cnt = 0 #reset counter since last log
            prev_time = curr_time

    #print(x)
    #print(y)
    ax.plot(x,y,label=device)
ax.legend();
plt.savefig("graph.png", dpi=500)

        

        


