# Node Storage Directory

This directory is intended to be used as the base path for local blockchain database storage and state persistence. When running the node using the `--base-path ./node-storage` argument, Substrate will create internal databases and keystores here to persist state between runs.

**Note:** This directory is ignored by Git, with the exception of structural files like this one or `.gitkeep`, to prevent committing heavy blockchain database files.
