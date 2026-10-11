set -e
external_size=$(nm __OBJ1__ | grep -c ' U _ZNK3BoxIcE4sizeEv$' | tr -d '[:space:]')
local_compare=$(nm __OBJ1__ | grep -c ' W _ZNK3BoxIcE7compareEi$' | tr -d '[:space:]')
local_destructor=$(nm __OBJ1__ | grep -c ' W _ZN3BoxIcED2Ev$' | tr -d '[:space:]')
echo "size_external $external_size"
echo "compare_local_weak $local_compare"
echo "destructor_local_weak $local_destructor"
