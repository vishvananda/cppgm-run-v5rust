// N3485 7.1.5p9, 3.2p2: constexpr fixed v = id(fixed(150)) in a function; if
// the call is emitted, id must be defined in the object; expect exit 0
struct fixed { long raw; constexpr explicit fixed(long r) : raw(r) {} };
constexpr fixed id(fixed a) { return a; }
int main() { constexpr fixed v = id(fixed(150)); return v.raw == 150 ? 0 : 1; }
