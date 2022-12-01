import pygame
from pygame.locals import *
import glm

from OpenGL.GL import *
from OpenGL.GLU import *

import numpy
import matplotlib.pyplot as plt

import argparse
import operator
import time
import sys

verticies = (
    glm.vec3(1, -1, -1),
    glm.vec3(1, 1, -1),
    glm.vec3(-1, 1, -1),
    glm.vec3(-1, -1, -1),
    glm.vec3(1, -1, 1),
    glm.vec3(1, 1, 1),
    glm.vec3(-1, -1, 1),
    glm.vec3(-1, 1, 1)
)

edges = (
    (0,1),
    (0,3),
    (0,4),
    (2,1),
    (2,3),
    (2,7),
    (6,3),
    (6,4),
    (6,7),
    (5,1),
    (5,4),
    (5,7)
)

parser = argparse.ArgumentParser(description="A python + OpenGL based visualizer that renders a NS3 UAV swarm in 3D")
parser.add_argument("--graph-only", help="Disables showing a window and rendering the UAVs. Generates only the MAD graph", action="store_true")
parser.add_argument("--skip-mad", help="Disables computing the MAD distance over time graph", action="store_true")
parser.add_argument("--file", default="positions.csv", help="A positions CSV file to read")
parser.add_argument("--mad-file", default="mad_graph.png", help="Specifies what the output filename should be")
args = parser.parse_args()



#Mean absloute deviation
def mad(arr, axis=None, keepdims=True):
    mean = numpy.mean(arr, axis=axis, keepdims=True)
    mad = numpy.mean(numpy.abs(arr-mean),axis=axis, keepdims=keepdims)
    return mad


def render_cube(offset=glm.vec3(0,0,0), size=1.0, color=glm.vec3(1, 1, 1)):
    glBegin(GL_LINES)
    glColor3f(color.x, color.y, color.z)
    for edge in edges:
        for vertex in edge:
            tmp = verticies[vertex] * size / 2.0 + offset
            glVertex3f(tmp.x, tmp.y, tmp.z)
    glEnd()



def render_ray(offset, ray):
    glBegin(GL_LINES)
    glColor3f(1.0, 0.0, 0.0)
    glVertex3f(offset.x, offset.y, offset.z)
    glVertex3f(ray.x, ray.y, ray.z)
    glEnd()


def lerp(a, b, f):
    # Convert the 0-1 range into a value in the right range.
    return a + ((b - a) * f)


def normalize(a, b, value):
    return float(value - a) / float(b - a)


def map(value, leftMin, leftMax, rightMin, rightMax):
    # Figure out how 'wide' each range is
    f = normalize(leftMin, leftMax, value)

    return lerp(rightMin, rightMax, f)

def parse_csv_line(line):
    parts = line.split(",")
    if parts[0] == "color":
        result = {}
        result["time"] = float(parts[1])
        result["ip_address"] = parts[2]
        result["red"] = float(parts[3])
        result["green"] = float(parts[4])
        result["blue"] = float(parts[5])

        result["color"] = glm.vec3(result["red"], result["green"], result["blue"])
        return result

    else:
        result = {}
        result["time"] = float(parts[0])
        result["ip_address"] = parts[1]
        result["x"] = float(parts[2])
        result["y"] = float(parts[3])
        result["z"] = float(parts[4])
        return result


def main():
    file_name = args.file
    print("Opening input CSV file: " + file_name)
    csv_file = open(file_name, "r")
    raw_lines = csv_file.read().splitlines()
    csv_file.close()
    csv_header = raw_lines[0]

    render_uavs = not args.graph_only
    use_mad = not args.skip_mad

    lines = []
    colors = []
    for line in raw_lines[1:]:
        #Parse raw lines and stick into lines
        parsed = parse_csv_line(line)
        if "color" in parsed:
            colors.append(parsed)
        else:
            parsed["pos"] = glm.vec3(parsed["x"], parsed["y"], parsed["z"])
            lines.append(parsed)

    uavs = set()
    for line in lines:
        uavs.add(line["ip_address"])

    print("Loaded {} uav's from file {}".format(len(uavs), file_name))

    #Mapping between a uav id and the index of the latest line that assert's a uav's position that is before the current re-simulation's time
    #Used for interpolating each uav's position each frame
    uav_last_index = {}
    uav_before_indices = {}
    uav_after_indices = {}

    for uav in uavs:
        #Start searching for times at the start of the file
        uav_last_index[uav] = 0
        uav_before_indices[uav] = None
        uav_after_indices[uav] = None

    uav_colors = {}

    for i, uav in enumerate(uavs):
        #All UAV's are white by default
        uav_colors[uav] = glm.vec3(0.0, 0.0, 0.0)
        #uav_colors[uav] = glm.vec3(0, (i / len(uavs))**1.5, 0)

    target_fps = 144
    if render_uavs:
        pygame.init()
        display = (1440, 810)
        pygame.display.set_mode(display, DOUBLEBUF | OPENGL)
        clock=pygame.time.Clock()
        pygame.mouse.set_visible(False)
        pygame.event.set_grab(True)

        simulation_speed = 1
        sensitivity = 0.75 * 1.0 / target_fps
        
        move_speed = 15.0 * 1.0 / target_fps

        gluPerspective(70, (display[0]/display[1]), 0.01, 500.0)

        #The position of the camera in 3d space
        camera_pos = glm.vec3(3.0, 0.0, 10.0)

        #A unit vector pointing where the camera is looking - relative to the position of the camera
        camera_forward = glm.normalize(glm.vec3(-0.3, 0.0, -1.0))
        keymap = {}

        for i in range(0, 256):
            keymap[i] = False

    else:
        last_time_print = 0

    last_time = time.time()
    delta_time = None
    simulation_time = 0.0
    exit_loop = False
    
    mad_times = []
    mad_values = []
    mad_speeds = []

    list_uavs = list(uavs)
    smallest = -1
    for i in range(1, len(list_uavs)):
        best_num = int(list_uavs[smallest].split(".")[3])
        current_num = int(list_uavs[i].split(".")[3])
        if smallest == -1 or current_num < best_num:
            print("setting {} {}".format(current_num, best_num))
            smallest = i
    
    print("best num " + str(list_uavs[smallest]))
    central = list_uavs[smallest]
    
    while True:

        if render_uavs:
            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    exit_loop = True
                    break
                    
                camera_up = glm.vec3(0.0, 1.0, 0.0)
                camera_right = glm.cross(camera_forward, camera_up)
                if event.type == pygame.MOUSEMOTION:
                    mouse_move = pygame.mouse.get_rel()
                    camera_forward = glm.rotate(camera_forward, -mouse_move[0] * sensitivity, camera_up)
                    camera_forward = glm.rotate(camera_forward, -mouse_move[1] * sensitivity, camera_right)
                    #cameraLook.rotate(

                if event.type == pygame.KEYDOWN:
                    keymap[event.scancode] = True
                    if (event.scancode == pygame.KSCAN_ESCAPE):
                        exit_loop = True
                        break

                if event.type == pygame.KEYUP:
                    keymap[event.scancode] = False

            if exit_loop:
                break

            camera_move = glm.vec3(0.0)
            if keymap[pygame.KSCAN_W]:
                camera_move += camera_forward
            
            if keymap[pygame.KSCAN_S]:
                camera_move -= camera_forward
            
            if keymap[pygame.KSCAN_A]:
                camera_move -= camera_right
            
            if keymap[pygame.KSCAN_D]:
                camera_move += camera_right
             
            if keymap[pygame.KSCAN_SPACE]:
                camera_move += camera_up
              
            if keymap[pygame.KSCAN_LSHIFT]:
                camera_move -= camera_up

            if glm.length(camera_move) > 0.0:
                camera_pos += glm.normalize(camera_move) * move_speed
        

            glPushMatrix()

            #glRotatef(1, 3, 1, 1)
            glClearColor(1.0, 1.0, 1.0, 1.0)
            glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT)

            gluLookAt(camera_pos.x, camera_pos.y, camera_pos.z, camera_pos.x + camera_forward.x, camera_pos.y + camera_forward.y, camera_pos.z + camera_forward.z, 0.0, 1.0, 0.0)


            #Re compute the current colors using the current time
            #for color in colors:
            #    if simulation_time > color["time"]:
            #        #uav_colors[color["ip_address"]] = color["color"]
            #        uav_colors[color["ip_address"]] = color["color"]

        #Counts the number of UAV's that have no future position assignments
        uav_done_with_pos_cout = 0
        for uav in uavs:
            glEnable(GL_LINE_SMOOTH)
            glLineWidth(3)

            #We need to find 2 lines in the list of all lines that tell us this uav's position before and after this re rendering's time step
            #We will the interpolate these two time points with the uav's position and our middle time in this rendering, allowing us
            #to have a smooth re-rendering of the swarm
            search_index = uav_last_index[uav]
            #We need to find the latest time a position was set in the csv file that is before _simulation_time_.
            #If this doesn't exist then we skip this uav since it doesn't have a position yet

            before_index = None
            after_index = None
            while True:
                if search_index >= len(lines):
                    uav_done_with_pos_cout += 1
                    break

                data = lines[search_index]
                if data["ip_address"] == uav:
                    #We found a line that talks about this uav
                    if data["time"] <= simulation_time:
                        before_index = search_index
                    else:
                        after_index = search_index
                        break
                search_index += 1

            if before_index != None:
                uav_last_index[uav] = before_index
            else:
                uav_last_index[uav] = search_index

            uav_before_indices[uav] = before_index
            uav_after_indices[uav] = after_index

            #print("Before {}, after {}".format(before_index, after_index))

            if render_uavs:
                if before_index == None and after_index == None:
                    #We have no position data nothing to do
                    continue
                elif before_index != None and after_index == None:
                    #There are no more data points after this point in the simulation. Lock uav's to their last known position
                    pos = lines[before_index]["pos"]
                elif before_index == None and after_index != None:
                    #There are no data points before here so for all we know the UAV was always here (should be unlikely in pratice)
                    pos = lines[after_index]["pos"]
                else:
                    #We have data points before and after so interpolate!
                    a = lines[before_index]["pos"]
                    a_time = lines[before_index]["time"]
                    b = lines[after_index]["pos"]
                    b_time = lines[after_index]["time"]

                    f = normalize(a_time, b_time, simulation_time)
                    pos = lerp(a, b, f)
                    #print("sim {}, a is {}, b is {}, f {}".format(simulation_time, a_time, b_time, f))
                color = uav_colors[uav]

                render_cube(offset=pos, size=0.5, color=color)
            

        if render_uavs:
            #Show origin as green
            render_cube(glm.vec3(0, 0, 0), 0.1, color=glm.vec3(0, 1, 0))
            pygame.display.flip()
            glPopMatrix()
        
        if use_mad:
            #Compute mad of distances
            central_last = lines[uav_last_index[central]]["pos"]

            distances = []
            speed_total = 0.0
            counted_for_speed = 0
            for i in range(0, len(list_uavs)):
                uav_id = list_uavs[i]
                if uav_before_indices[uav_id] != None and uav_after_indices[uav_id] != None and delta_time != None:
                    #Only compute speed when we have a initial and final position and when we have a delta time
                    a_pos =  lines[uav_before_indices[uav_id]]["pos"]
                    a_time = lines[uav_before_indices[uav_id]]["time"]

                    b_pos =  lines[uav_after_indices[uav_id]]["pos"]
                    b_time = lines[uav_after_indices[uav_id]]["time"]

                    speed = abs(glm.length(b_pos - a_pos) / (b_time - a_time))
                    speed_total += speed
                    counted_for_speed += 1

                if i == smallest:
                    #Dont compute the distance from the central node to the central node
                    continue

                last_pos = lines[uav_last_index[uav_id]]["pos"]
                distance = glm.length(central_last - last_pos)
                distances.append(distance)

            mean = numpy.mean(distances)
            mad_value = mad(distances)[0]
            mad_percent = mad_value * mean * 100
            mad_times.append(lines[uav_last_index[central]]["time"])
            mad_values.append(mad_percent)
            if counted_for_speed == 0:
                mad_speeds.append(0)
            else:
                mad_speeds.append(speed_total / counted_for_speed)

        if render_uavs:

            clock.tick(target_fps)
            now = time.time()
            delta_time = (now - last_time) * simulation_speed
            last_time = now

        else:
            #Fixed simulation steps when we arent rendering in real time
            delta_time = 1.0 / target_fps
            if simulation_time - last_time_print > 15:
                print("Simulation at " + str(simulation_time))
                last_time_print = simulation_time

            if uav_done_with_pos_cout == len(uavs):
                print("Finished MAD data collection")
                break

        #simulation_time += delta_time
        if simulation_time < 25:
            simulation_time += delta_time
        #print("time at {}, delta {}".format(simulation_time, delta_time))

    if use_mad:
        #Simulation done. Graph mad
        print("Saving graph to: " + args.mad_file)
        plt.scatter(mad_times, mad_values, s=2)
        plt.xlabel("Time (s)")
        plt.ylabel("MAD %")
        plt.title("Mean Absolute Deviation Distance as Percent of the Mean vs Time")
        plt.savefig(args.mad_file, dpi=500)

        plt.clf()
        plt.scatter(mad_times, mad_speeds, s=2)
        plt.xlabel("Time (s)")
        plt.ylabel("Average Network Speed")
        plt.title("Average Network Speed vs Time")
        plt.savefig("speed-" + args.mad_file, dpi=500)

        with open("all-data.csv", "w") as f:
            f.write("Time,Mad Distance,Average Velocity")
            for i in range(0, len(mad_times)):
                currrent_time = mad_times[i]
                mad_distance = mad_values[i]
                speed = mad_speeds[i]
                f.write("{},{},{}\n".format(currrent_time, mad_distance, speed))


    pygame.quit()
    sys.exit()


main()

