set -eu
nm -u "__OBJ1__" | grep -q 'cxa_call_unexpected'
nm -u "__OBJ1__" | grep -q 'cxa_throw'
nm -u "__OBJ1__" | grep -q 'cxa_begin_catch'
nm -u "__OBJ1__" | grep -q 'cxa_end_catch'
nm -u "__OBJ1__" | grep -q 'gxx_personality'
echo dynamic_exception_spec_surface_ok
