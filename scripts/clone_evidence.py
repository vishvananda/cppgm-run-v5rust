#!/usr/bin/env python3
import os
import sys
os.execv(sys.executable, [sys.executable, "/home/vishvananda/work/v5rust-control/clone_evidence.py", *sys.argv[1:]])
