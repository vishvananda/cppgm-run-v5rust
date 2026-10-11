set -eu
if nm -u "__OBJ1__" | grep -q '_\{0,1\}alloca'; then
  echo unexpected_alloca_symbol
  exit 1
fi
echo host_builtin_alloca_surface_ok 1
