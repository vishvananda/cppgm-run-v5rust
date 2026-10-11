set -e
external=$(nm __OBJ1__ | grep -c ' U _ZN3TagIcE2idE$' | tr -d '[:space:]')
defined=$(nm __OBJ1__ | grep -c ' [DdVvBb] _ZN3TagIcE2idE$' | tr -d '[:space:]')
echo "char_id_external $external"
echo "char_id_defined_here $defined"
