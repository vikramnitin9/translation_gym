FROM coreutils:latest

RUN ["rm", "-f", "/app/src/tail"]
RUN ["sudo", "rm", "-f", "/user/bin/tail"]

# Failing:
# /app/tests/tail/follow-name.sh
# /app/tests/tail/follow-stdin.sh
# /app/tests/tail/inotify-dir-recreate.sh
# /app/tests/tail/pid-pipe.sh
# /app/tests/tail/pipe-f.sh


CMD ["bash", "-c", "make TESTS=\"/app/tests/tail/append-only.sh \
/app/tests/tail/assert-2.sh \
/app/tests/tail/assert.sh \
/app/tests/tail/big-4gb.sh \
/app/tests/tail/descriptor-vs-rename.sh \
/app/tests/tail/end-of-device.sh \
/app/tests/tail/F-headers.sh \
/app/tests/tail/flush-initial.sh \
/app/tests/tail/F-vs-missing.sh \
/app/tests/tail/F-vs-rename.sh \
/app/tests/tail/inotify-hash-abuse2.sh \
/app/tests/tail/inotify-hash-abuse.sh \
/app/tests/tail/inotify-only-regular.sh \
/app/tests/tail/inotify-race2.sh \
/app/tests/tail/inotify-race.sh \
/app/tests/tail/inotify-rotate-resources.sh \
/app/tests/tail/inotify-rotate.sh \
/app/tests/tail/overlay-headers.sh \
/app/tests/tail/pid.sh \
/app/tests/tail/pipe-f2.sh \
/app/tests/tail/proc-ksyms.sh \
/app/tests/tail/retry.sh \
/app/tests/tail/start-middle.sh \
/app/tests/tail/symlink.sh \
/app/tests/tail/tail-c.sh \
/app/tests/tail/tail-n0f.sh \
/app/tests/tail/tail.pl \
/app/tests/tail/tail-sysfs.sh \
/app/tests/tail/truncate.sh \
/app/tests/tail/wait.sh\" \
SUBDIRS=. VERBOSE=yes check && \
    { [ -f \"/app/instrumentation/instrumented.json\" ] \
      && sudo cp /app/instrumentation/instrumented.json \"/instrumentation/\"; true; }"]
