FROM coreutils:latest

RUN ["rm", "-f", "/app/src/tail"]
RUN ["sudo", "rm", "-f", "/user/bin/tail"]

# Failing:
# /app/tests/tail/follow-name.sh
# /app/tests/tail/follow-stdin.sh
# /app/tests/tail/inotify-dir-recreate.sh
# /app/tests/tail/pid-pipe.sh
# /app/tests/tail/pipe-f.sh


CMD ["bash", "-c", "make TESTS=\"tests/tail/append-only.sh \
tests/tail/assert-2.sh \
tests/tail/assert.sh \
tests/tail/big-4gb.sh \
tests/tail/descriptor-vs-rename.sh \
tests/tail/end-of-device.sh \
tests/tail/F-headers.sh \
tests/tail/flush-initial.sh \
tests/tail/F-vs-missing.sh \
tests/tail/F-vs-rename.sh \
tests/tail/inotify-hash-abuse2.sh \
tests/tail/inotify-hash-abuse.sh \
tests/tail/inotify-only-regular.sh \
tests/tail/inotify-race2.sh \
tests/tail/inotify-race.sh \
tests/tail/inotify-rotate-resources.sh \
tests/tail/inotify-rotate.sh \
tests/tail/overlay-headers.sh \
tests/tail/pid.sh \
tests/tail/pipe-f2.sh \
tests/tail/proc-ksyms.sh \
tests/tail/retry.sh \
tests/tail/start-middle.sh \
tests/tail/symlink.sh \
tests/tail/tail-c.sh \
tests/tail/tail-n0f.sh \
tests/tail/tail.pl \
tests/tail/tail-sysfs.sh \
tests/tail/truncate.sh \
tests/tail/wait.sh\" \
SUBDIRS=. VERBOSE=yes check && \
    { [ -f \"/app/instrumentation/instrumented.json\" ] \
      && sudo cp /app/instrumentation/instrumented.json \"/instrumentation/\"; true; }"]
