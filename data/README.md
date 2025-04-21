## Datasets

### Adding a new dataset

Currently we support only the C language. Each dataset must have the following:
- a C code repository containing a Makefile. If your code uses Automake, CMake, etc, make sure you run the initial steps to generate a Makefile.
- a Docker image for running tests. The Docker image will be run as a container, and the compiled executable will be mounted as a volume `/executable`.
    - You should have a `CMD` that runs the tests, and returns with exit code 0 for success and non-zero for failure.
    - Further, the code will generate a file `instrumented.json` when run, in the directory given by the environment variable `INSTRUMENTATION_PATH`. As part of your Docker `CMD`, you should copy `$INSTRUMENTATION_PATH/instrumented.json` to the mounted volume `/instrumented`. You could alternatively just set `$INSTRUMENTATION_PATH` to `/instrumented`, but this could cause permission issues depending on your configuration.

Once you have both of the above, create a dataset entry in `datasets.json`. For an example of this, take a look at the `toy` dataset and its Dockerfile, located at (`data/toy/tests/Dockerfile`)[data/toy/tests/Dockerfile].