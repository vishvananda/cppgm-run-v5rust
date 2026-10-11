#!/bin/sh

cppgm_run_debugger_command() {
  "$CPPGM_DEBUGGER_BIN" "$@" 2>&1
}

cppgm_try_lldb() {
  if [ -x /usr/bin/lldb ]; then
    CPPGM_DEBUGGER_KIND=lldb
    CPPGM_DEBUGGER_BIN=/usr/bin/lldb
    return 0
  fi
  cppgm_lldb_bin=$(command -v lldb 2>/dev/null) || return 1
  CPPGM_DEBUGGER_KIND=lldb
  CPPGM_DEBUGGER_BIN=$cppgm_lldb_bin
}

cppgm_try_gdb() {
  cppgm_gdb_bin=$(command -v gdb 2>/dev/null) || return 1
  CPPGM_DEBUGGER_KIND=gdb
  CPPGM_DEBUGGER_BIN=$cppgm_gdb_bin
}

cppgm_select_debugger() {
  cppgm_request=${CPPGM_DEBUGGER:-auto}
  case "$cppgm_request" in
    auto)
      cppgm_sys=$(uname -s 2>/dev/null || echo unknown)
      case "$cppgm_sys" in
        Darwin)
          cppgm_try_lldb || cppgm_try_gdb || {
            echo "missing debugger backend" >&2
            return 1
          }
          ;;
        *)
          cppgm_try_gdb || cppgm_try_lldb || {
            echo "missing debugger backend" >&2
            return 1
          }
          ;;
      esac
      ;;
    lldb)
      cppgm_try_lldb || {
        echo "lldb not found" >&2
        return 1
      }
      ;;
    gdb)
      cppgm_try_gdb || {
        echo "gdb not found" >&2
        return 1
      }
      ;;
    *)
      echo "unsupported debugger backend: $cppgm_request" >&2
      return 1
      ;;
  esac
}

cppgm_run_debugger_frame_and_vars() {
  cppgm_exe=$1
  cppgm_src=$2
  cppgm_line=$3
  cppgm_vars=$4

  case "$CPPGM_DEBUGGER_KIND" in
    lldb)
      "$CPPGM_DEBUGGER_BIN" --batch "$cppgm_exe" \
        -o "breakpoint set --file source.cpp --line $cppgm_line" \
        -o run \
        -o 'frame info' \
        -o "frame variable $cppgm_vars" 2>&1 |
        sed -E \
          -e "s#${cppgm_exe}#sample#g" \
          -e "s#${cppgm_src}#source.cpp#g" \
          -e 's/0x[0-9a-f]+/0xADDR/g' |
        awk -v vars="$cppgm_vars" '
          BEGIN {
            n = split(vars, list, " ");
            for (i = 1; i <= n; ++i) {
              if (list[i] != "") {
                allowed[list[i]] = 1;
              }
            }
          }
          /^frame #0:/ {
            raw = $0;
            fn = raw;
            sub(/^.*`/, "", fn);
            sub(/ at .*/, "", fn);
            sub(/\(.*/, "", fn);
            loc = raw;
            sub(/^.* at /, "", loc);
            split(loc, parts, ":");
            file = parts[1];
            sub(/^.*\//, "", file);
            print "frame function=" fn " file=" file " line=" parts[2];
            next;
          }
          /^\([^)]*\) [A-Za-z_][A-Za-z0-9_]* = / {
            raw = $0;
            sub(/^\([^)]*\) /, "", raw);
            name = raw;
            sub(/ = .*/, "", name);
            if (!(name in allowed)) {
              next;
            }
            value = raw;
            sub(/^[^=]* = /, "", value);
            print "var " name "=" value;
          }'
      ;;
    gdb)
      cppgm_run_debugger_command --batch -q "$cppgm_exe" \
        -ex 'set pagination off' \
        -ex 'set confirm off' \
        -ex 'set startup-with-shell off' \
        -ex 'set breakpoint pending on' \
        -ex "break source.cpp:$cppgm_line" \
        -ex run \
        -ex frame \
        -ex 'info args' \
        -ex 'info locals' \
        -ex quit |
        awk -v vars="$cppgm_vars" '
          BEGIN {
            n = split(vars, list, " ");
            for (i = 1; i <= n; ++i) {
              if (list[i] != "") {
                allowed[list[i]] = 1;
              }
            }
          }
          /^#0[ \t]/ {
            raw = $0;
            fn = raw;
            sub(/^#0[ \t]+/, "", fn);
            sub(/ at .*/, "", fn);
            sub(/[ \t]*\(.*/, "", fn);
            loc = raw;
            sub(/^.* at /, "", loc);
            split(loc, parts, ":");
            file = parts[1];
            sub(/^.*\//, "", file);
            print "frame function=" fn " file=" file " line=" parts[2];
            next;
          }
          /^[A-Za-z_][A-Za-z0-9_]* = / {
            name = $0;
            sub(/ = .*/, "", name);
            if (!(name in allowed)) {
              next;
            }
            value = $0;
            sub(/^[^=]* = /, "", value);
            print "var " name "=" value;
          }'
      ;;
  esac
}

cppgm_run_debugger_pending_breakpoint() {
  cppgm_exe=$1
  cppgm_src=$2
  cppgm_line=$3

  case "$CPPGM_DEBUGGER_KIND" in
    lldb)
      "$CPPGM_DEBUGGER_BIN" --batch "$cppgm_exe" \
        -o "breakpoint set --file source.cpp --line $cppgm_line" \
        -o 'breakpoint list' 2>&1 |
        sed -E \
          -e "s#${cppgm_exe}#sample#g" \
          -e "s#${cppgm_src}#source.cpp#g" |
        awk '
          /^[0-9]+: file = / {
            raw = $0;
            sub(/^[0-9]+: file = /, "", raw);
            quote = sprintf("%c", 39);
            gsub(quote, "", raw);
            split(raw, parts, ",");
            file = parts[1];
            sub(/^ */, "", file);
            line_part = parts[2];
            sub(/^ line = /, "", line_part);
            print "breakpoint status=pending file=" file " line=" line_part;
          }'
      ;;
    gdb)
      cppgm_run_debugger_command --batch -q "$cppgm_exe" \
        -ex 'set pagination off' \
        -ex 'set confirm off' \
        -ex 'set breakpoint pending on' \
        -ex "break source.cpp:$cppgm_line" \
        -ex 'info breakpoints' \
        -ex quit |
        awk '
          /<PENDING>/ {
            raw = $NF;
            split(raw, parts, ":");
            file = parts[1];
            sub(/^.*\//, "", file);
            print "breakpoint status=pending file=" file " line=" parts[2];
          }'
      ;;
  esac
}
