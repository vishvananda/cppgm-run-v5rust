set -e
deleting=$(nm __OBJ1__ | grep -c '_ZN4baseD0B2v1Ev$' | tr -d '[:space:]')
complete=$(nm __OBJ1__ | grep -c '_ZN4baseD1B2v1Ev$' | tr -d '[:space:]')
echo "base_deleting_entry $deleting"
echo "base_complete_entry $complete"
