import argparse
import shutil
import os
from pathlib import Path
import subprocess
import json
import networkx as nx
import datetime
from typing import List, Dict, Tuple
import textwrap
import docker
import socket
import pathlib

def prRed(skk): print("\033[91m {}\033[00m" .format(skk))
def prGreen(skk): print("\033[92m {}\033[00m" .format(skk))
def prCyan(skk): print("\033[96m {}\033[00m" .format(skk))
def prYellow(skk): print("\033[93m {}\033[00m" .format(skk))
def prLightPurple(skk): print("\033[94m {}\033[00m" .format(skk))
def prLightGray(skk): print("\033[97m {}\033[00m" .format(skk))

class CompileException(Exception):
    pass

class RunException(Exception):
    pass

def run(command, verbose=False):

    try:
        result = subprocess.run(
            command,
            shell=True,
            timeout=120,
            stderr=subprocess.PIPE,
            stdout=subprocess.PIPE,
        )
        if result.returncode != 0:
            exec_output = result.stderr.decode('utf-8', errors='ignore')
            if exec_output.strip() == '':
                exec_output = result.stdout.decode('utf-8', errors='ignore')
            if verbose:
                prLightGray(exec_output)
            raise RunException(exec_output)
    except subprocess.TimeoutExpired:
        raise RunException("Timeout")
    except subprocess.CalledProcessError as e:
        raise RunException(e.output.decode('utf-8', errors='ignore'))
    except Exception as e:
        raise RunException(str(e))
    
    if verbose:
        prLightGray(result.stdout.decode('utf-8', errors='ignore'))

def safe_load_json(filepath):
    with open(filepath, "rb") as f:  # Open in binary mode
        raw_bytes = f.read()  # Read raw data

    json_str = raw_bytes.decode("utf-8", errors="backslashreplace").strip()

    if json_str.endswith(","):  
        json_str = json_str[:-1]  # Remove trailing comma
    if not json_str.startswith("["):
        json_str = "[" + json_str
    if not json_str.endswith("]"):
        json_str = json_str + "]"

    def sanitize(c):
        if c == '\\':
            return '\\\\'
        else:
            return c

    json_clean = ''.join(sanitize(c) for c in json_str)
    try:
        data = json.loads(json_clean.strip(), strict=False)
        return data
    except:
        prRed(f"Failed to load JSON from {filepath}")
        return None

def to_host_path(path):
    if not os.path.exists('/.dockerenv'):
        return path
    
    # We are inside a Docker container
    client          = docker.DockerClient(base_url='unix:///var/run/docker.sock')
    container_id    = socket.gethostname()
    container       = client.containers.get(container_id)
    mounts          = container.attrs['Mounts']

    mount_path_len = 0
    host_path = None
    # Find the mount that is the longest prefix of path
    for mount in mounts:
        if path.is_relative_to(Path(mount['Destination'])):
            if len(Path(mount['Destination']).parts) > mount_path_len:
                mount_path_len = len(Path(mount['Destination']).parts)
                host_path = Path(mount['Source'])/path.relative_to(Path(mount['Destination']))
    return host_path