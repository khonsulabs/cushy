# Cushy User Guide

The Cushy user guide contains examples that are generated using Cushy's virtual
recorder functionality. To build this guide:

1. Install mdbook and mdbook-variables
2. Capture examples:

   ```sh
   CAPTURE=1 cargo test -p guide-examples --examples
   ```

3. Build the guide:

   ```sh
   mdbook build guide
   ```
