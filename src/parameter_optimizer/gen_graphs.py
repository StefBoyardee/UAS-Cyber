#!/bin/python

import argparse
import os
import sys
import json
import numpy as np
import matplotlib.pyplot as plt
import bisect
from pathlib import Path
from sklearn.linear_model import LinearRegression
from scipy.optimize import curve_fit
import pandas as pd
import seaborn as sns

from matplotlib.patches import Patch
import matplotlib as mpl
# Use true LaTeX and bigger font
#mpl.rc('text', usetex=True)
# Include packages `amssymb` and `amsmath` in LaTeX preamble
# as they include extended math support (symbols, envisonments etc.)
#mpl.rcParams['text.latex.preamble'] = [r"\usepackage{amssymb}",
#                                       r"\usepackage{amsmath}"]

parser = argparse.ArgumentParser(description='Exports json simulation data to graphs')
parser.add_argument('file', default=None,
                    help='The file or directory to export. If directory, all matching json files will be exported')
parser.add_argument('--prefix', default="",
                    help='A prefix which is written before each exported graph')
parser.add_argument('--one-graph', action='store_true',
                    help='Export a single graph with all the data points')

args = parser.parse_args()
if not os.path.exists(args.file):
    print("ERROR: File {} does not exist!".format(args.file))
    sys.exit(1)

fig = None
ax = None

series_format_order = ""
d_values_map = {}
average_origin_distance_keys = []
average_origin_distance_values = []

x_param_expected = "r"
y_param_expected = "a"

def export_single(path, prefix):
    '''Exports the data from a single json file to a graph
    '''

    global fig
    global ax
    global series_format_order
    global d_values_map
    # Called multiple times with a different figure each time
    if not args.one_graph:
        fig = plt.figure()
    else:
        if fig == None:
            fig = plt.figure()
            print("Set figure")

    obj = json.load(open(path))
    params = obj["params"]

    x_param = params[1]["name"]
    y_param = params[0]["name"]
    if x_param.strip() != x_param_expected:
        print("x parameter is not " + x_param_expected + ": got " + x_param)
        sys.exit(1)

    if y_param.strip() != y_param_expected:
        print("y parameter is not " + y_param_expected + ": got " + y_param)
        sys.exit(1)

    #print("X-axis param is " + x_param)
    #print("Y-axis param is " + y_param)

    results = obj["results"]
    errors = []
    run_params = []
    x_values = []
    y_values = []
    for result in results:
        params = result["parameters"]
        errors.append(result["fitness"])
        run_params.append(params)
        x_values.append(params[x_param])
        y_values.append(params[y_param])

    std_range = 1.5
    x_mean = np.mean(x_values)
    x_stddev = np.std(x_values)
    y_mean = np.mean(y_values)
    y_stddev = np.std(y_values)

    x_min = max(x_mean - x_stddev * std_range, 0.0)
    x_max = x_mean + x_stddev * std_range
    y_min = max(y_mean - y_stddev * std_range, 0.0)
    y_max = y_mean + y_stddev * std_range

    # Only display values within `std_range` standard deviations of the mean
    i = 0
    while True:
        if i >= len(x_values):
            break
        remove = False
        if x_values[i] < x_min or x_values[i] > x_max:
            remove = True
        if y_values[i] < y_min or y_values[i] > y_max:
            remove = True
        if remove:
            x_values.pop(i)
            y_values.pop(i)
            errors.pop(i)
        else:
            i += 1

    # 256 numbers which will be used to query value for that percentile
    hundred_numbers = np.arange(0.0, 100, step=100/256)
    percentiles = np.percentile(errors, hundred_numbers)
    colors = []
    for error in errors:
        index = bisect.bisect_left(percentiles, error)
        colors.append((index / 256, (256 - index) / 256, 0.3))

    #Remove bad data points if we are rendering to a single graph
    if args.one_graph:
        colors = None
        i = 0
        while True:
            if i >= len(x_values):
                break
            # How good is this data point?
            index = bisect.bisect_left(percentiles, errors[i])
            # Remove if not in the to 20%
            if index > 20 * (256 / 100):
                x_values.pop(i)
                y_values.pop(i)
                errors.pop(i)
            else:
                i += 1

    if not args.one_graph:
        ax = fig.add_subplot(111)
    else:
        if ax == None:
            print("Made subplot")
            ax = fig.add_subplot(111)
            ax.spines['right'].set_visible(False)
            ax.spines['top'].set_visible(False)

    #ax = fig.axes()
    ax.scatter(x_values, y_values, s=2, c=colors)

    # Do linear regression and display line
    weights = []
    for error in errors:
        if error > percentiles[int(10 * 256 / 100)]:
            # Give 0 weight to data points below the 90th percentile
            # Indexing looks odd because `len(percentiles == 256`
            weights.append(0)
        else:
            weights.append(1000.0 / error)

    feed_x = np.array(x_values).reshape(-1, 1)
    feed_y = np.array(y_values)

    model = LinearRegression()
    model.fit(feed_x, feed_y, sample_weight=weights)

    x_new = np.linspace(x_min, x_max, 200)
    y_new = model.predict(x_new[:, np.newaxis])

    parent_path = Path(path).parent
    label_path = os.path.join(parent_path, "label.txt")
    if os.path.exists(label_path):
        f = open(label_path) 
        label = f.read().strip()
        if len(label) > 30:
            print("WARN: Label file " + label_path + " is over 30 characters. Data may be obstructed on graph")
        if len(label) == 0:
            print("ERROR: Label file " + label_path + " empty")
            sys.exit(1)
        else:
            parts = list(map(lambda x: x.split("="), label.strip().split(" ")))
            vars = {}
            for tup in list(parts):
                vars[tup[0]] = float(tup[1])

            d = vars['d']
            tx = vars['tx']
            #Write data points
            if d in d_values_map:
                obj = d_values_map[d]
            else:
                obj = {
                'x_values': [],
                'y_values': [],
            }
            obj['x_values'].extend(x_values)
            obj['y_values'].extend(y_values)

            d_values_map[d] = obj

    else:
        print("ERROR: Missing label " + label_path)
        print("Edit file with desired label and re-run to fix")
        sys.exit(1)

    #Use same weights from regression so that we only mean reasonable points
    x_mean = np.average(x_values, None, weights)
    y_mean = np.average(y_values, None, weights)
    average_origin_distance_keys.append({'d': d, 'tx': tx})
    average_origin_distance_values.append(np.sqrt(x_mean * x_mean + y_mean * y_mean))

    ax.plot(x_new, y_new, label=label)
    ax.spines['right'].set_visible(False)
    ax.spines['top'].set_visible(False)

    print("y=", model.coef_, "x + ", model.intercept_, " for ", label)
    ax.set_xlabel(x_param)
    ax.set_ylabel(y_param)
    if not args.one_graph:

        ax.axis('tight')
        name = prefix + "hot_cold.png"
        fig.savefig(name)
        print("Wrote " + name)

def func_exp(x, a, b, c):
    #c = 0
    return a * np.exp(b * x) + c

def run_distance_analysis():
    '''Runs analysis for different values of D using the a and r data points collected from calling `export_single`
    '''
    global d_values_map
    d_values = []
    da_dr_values = []
    for d, values in d_values_map.items():

        #NO # Invert axis so that we get a quadratic relationship
        feed_x = np.array(values["x_values"]).reshape(-1, 1)
        feed_y = np.array(values["y_values"])

        model = LinearRegression()
        model.fit(feed_x, feed_y)
        r_2 = model.score(feed_x, feed_y)
        print("for d ", d, x_param_expected, "=", model.coef_, y_param_expected, " + ", model.intercept_, " (r^2", r_2, ")")

        d_values.append(d)
        da_dr_values.append(model.coef_[0])
 
        fig = plt.figure()
        ax = fig.add_subplot(111)
        ax.spines['right'].set_visible(False)
        ax.spines['top'].set_visible(False)

        ax.scatter(feed_x, feed_y)
        ax.set_xlabel(x_param_expected)
        ax.set_ylabel(y_param_expected)

        x_new = np.linspace(np.min(feed_x), np.max(feed_x), 200)
        y_new = model.predict(x_new[:, np.newaxis])

        ax.plot(x_new, y_new)
        ax.spines['right'].set_visible(False)
        ax.spines['top'].set_visible(False)

        fig.savefig("all" + str(d) + ".png")
        plt.clf()


    print("X: " + str(d_values))
    print("Y: " + str(da_dr_values))
    feed_x = np.array(d_values)
    feed_y = np.array(da_dr_values)

    xx = np.linspace(np.min(feed_x), np.max(feed_x), 1000)

    popt, pcov = curve_fit(func_exp, feed_x, feed_y, p0=[1, -0.5, 1], maxfev=5000)
    print("Raw regression results: " + str(popt))
    print("FINAL REGRESSION ", "da_dr=", popt[0], "*e^(", popt[1], "*d)+", popt[2])

    fig = plt.figure()
    ax = fig.add_subplot(111)
    ax.spines['right'].set_visible(False)
    ax.spines['top'].set_visible(False)
    ax.set_xlabel("d")
    ax.set_ylabel("da/dr")
    ax.scatter(feed_x, feed_y)
    ax.plot(xx, func_exp(xx, *popt))
    
    fig.savefig("overall.png")
    plt.clf()

multiple = os.path.isdir(args.file)

if multiple:
    to_export = []
    for dirpath, dirnames, files in os.walk(args.file):
        for name in files:
            parent_dir = os.path.basename(os.path.abspath(dirpath))
            if name.lower().endswith(".json"):
                to_export.append((dirpath, name, parent_dir))

    #Sort by the label values
    #to_export.sort(key=lambda x: open(os.path.join(x[0], "label.txt")).read())
    #Sort by the d value in 'd=XXX tx=...'
    to_export.sort(key=lambda x: float(open(os.path.join(x[0], "label.txt")).read().split("=")[1].split(" ")[0]))
    for i in range(0, len(to_export)):
        dirpath = to_export[i][0]
        name = to_export[i][1]
        parent_dir = to_export[i][2]

        export_single(os.path.join(dirpath, name), args.prefix + parent_dir)
else:
    export_single(args.file, args.prefix)

if args.one_graph:
    print("Saved figure")
    fig.legend(fontsize=10)
    fig.savefig(args.prefix + "hot_cold.png") 
    plt.clf()
    run_distance_analysis()

    #fig = plt.figure()
    print("keys " + str(average_origin_distance_keys))
    print("values " + str(average_origin_distance_values))
    #ax.set_xlabel("Simulation")
    #ax.set_ylabel("Distance from origin")

    # Determine the order average_origin_distance* lists should be in when we sort by `tx` so that
    # they appear sorted in the ledgend
    keys = []
    values = []
    for i, _key in sorted(enumerate(average_origin_distance_keys), key=lambda x: x[1]['tx']):
        keys.append(average_origin_distance_keys[i])
        values.append(average_origin_distance_values[i])

    # Maps values of d to the group index they will have on the graph
    d_indices = {}
    for i in range(0, len(keys)):
        vars = keys[i]
        if not vars['d'] in d_indices:
            d_indices[vars['d']] = len(d_indices)
    print("indices " + str(d_indices))

    tx_indices = {}
    for i in range(0, len(keys)):
        vars = keys[i]
        if not vars['tx'] in tx_indices:
            tx_indices[vars['tx']] = len(tx_indices)

    print("tx indices " + str(tx_indices))

    # Maps a tx value to an array len d_indices. Each value in the value of d in array
    # corrorsponds to the value of d given by d_indices
    data = np.zeros((len(tx_indices), len(d_indices)))
    print("before " + str(data))

    for i in range(0, len(keys)):
        vars = keys[i]
        d = vars['d']
        tx = vars['tx']
        value = values[i]
        data[tx_indices[tx]][d_indices[d]] = value

    print("after " + str(data))

    x_axis = np.arange(len(d_indices))
    tx_labels = list(tx_indices.keys())
    width = 0.27
    colors = ['r', 'y', 'c']
    ax = plt.subplot(111)
    for i in range(0, len(tx_indices)):
        pos_x = (i - len(tx_indices) / 2) * width
        ax.bar(x_axis + pos_x, data[i], width, label=tx_labels[i], align='edge', edgecolor='k', color=colors[i])

    ax.spines['right'].set_visible(False)
    ax.spines['top'].set_visible(False)
    plt.xticks(x_axis, d_indices.keys())
    plt.ylabel("Magnitude of a and r")
    plt.xlabel("Distance parameter (d)")
    plt.legend()
    plt.savefig("distances.png") 
    plt.clf()

