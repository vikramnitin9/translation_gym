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