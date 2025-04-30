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
import time

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

def run(command, timeout=120, verbose=False):

    if verbose:
        prCyan(f"Running command: {command}")
    try:
        result = subprocess.run(
            command,
            shell=True,
            timeout=timeout,
            stderr=subprocess.PIPE,
            stdout=subprocess.PIPE,
        )
        if result.returncode != 0:
            stderr = result.stderr.decode('utf-8', errors='ignore') if result.stderr else ''
            stdout = result.stdout.decode('utf-8', errors='ignore') if result.stdout else ''
            if verbose:
                prLightGray(f"STDOUT:\n{stdout}\nSTDERR:\n{stderr}")
            raise RunException(f"STDOUT:\n{stdout}\nSTDERR:\n{stderr}")
    except subprocess.TimeoutExpired:
        raise RunException("Timeout")
    except subprocess.CalledProcessError as e:
        raise RunException(e.output.decode('utf-8', errors='ignore'))
    except Exception as e:
        raise RunException(str(e))
    
    if verbose:
        prLightGray(result.stdout.decode('utf-8', errors='ignore'))
        
    return result.stdout.decode('utf-8', errors='ignore')

def safe_load_json(filepath):
    with open(filepath, "rb") as f:  # Open in binary mode
        raw_bytes = f.read()  # Read raw data

    json_str = raw_bytes.decode("latin-1", errors="backslashreplace").strip()

    if json_str.endswith(","):  
        json_str = json_str[:-1]  # Remove trailing comma
    if not json_str.startswith("["):
        json_str = "[" + json_str
    if not json_str.endswith("]"):
        json_str = json_str + "]"

    try:
        data = json.loads(json_str.strip(), strict=False)
        return data
    except:
        print(f"Failed to load JSON from {filepath}")
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


def get_last_modified_time(folder_path, filter_ext=None):
    """
    Returns the last modified time of a folder, considering all files and subfolders.

    Args:
    folder_path: The path to the folder.

    Returns:
    A float representing the last modified timestamp (seconds since epoch), or None if the folder does not exist.
    """
    if not os.path.exists(folder_path):
        return None

    max_mtime = 0
    for root, _, files in os.walk(folder_path):
        max_mtime = max(max_mtime, os.path.getmtime(root))
        for file in files:
            # Filter files based on the provided extension
            if filter_ext and not file.endswith(filter_ext):
                continue
            file_path = os.path.join(root, file)
            max_mtime = max(max_mtime, os.path.getmtime(file_path))
        
    return max_mtime