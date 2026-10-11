set -e
# The basic_ostream(basic_streambuf*) constructor the unit instantiates is
# defined here (one or two entries, the complete and the base), and no
# entry is left undefined for the library to supply.
defined=$(
  nm -an __OBJ2__ \
    | grep -v ' U ' \
    | c++filt \
    | grep 'basic_ostream<char, .*>::basic_ostream.*basic_streambuf<char,' \
    | wc -l \
    | tr -d '[:space:]'
)
undefined=$(
  nm -an __OBJ2__ \
    | grep ' U ' \
    | c++filt \
    | grep -c 'basic_ostream<char, .*>::basic_ostream.*basic_streambuf<char,' \
    || true
)
if [ "$defined" -lt 1 ] || [ "$defined" -gt 2 ] || [ "$undefined" -ne 0 ]; then
  echo "hosted_ostream_ctor_entries_defined 0"
  exit 1
fi
echo "hosted_ostream_ctor_entries_defined 1"
