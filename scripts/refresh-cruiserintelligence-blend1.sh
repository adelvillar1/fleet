#!/bin/bash
# Scheduled corpus-refresh wrapper: exports the blend1 lane's launch env (the
# spec src/lanes.rs produces) and runs the stdio JSON-RPC driver against the
# vendored blend1 lane child for the cruiserintelligence project.
set -e
FLEET=/Users/alejandrodelvillar/Projects/fleet
export BLEND1_PYTHONPATH="$FLEET/services/blend1/src"
export BLEND1_SYS1_PYTHONPATH="$FLEET/services/sys1/src"
export BLEND1_SEM1_PYTHONPATH="$FLEET/services/sem1/src"
export BLEND1_SDM1_PYTHONPATH="$FLEET/services/sdm1/src"
export BLEND1_GEN1_PYTHONPATH="$FLEET/services/gen1/src"
exec python3 -u "$FLEET/scripts/refresh-cruiserintelligence-blend1.py"
