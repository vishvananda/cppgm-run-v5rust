// N3485 8.5.4p3/8.5.3p5 + 12.2p5: a reference to const array bound to a
// braced-init-list ({ T(50), T(51) }) binds to a temporary array that persists
// with the reference; expect exit 0
struct T { int v; T(int x) : v(x) {} };
const T (&arr)[2] = { T(50), T(51) };
int main() { return arr[1].v == 51 ? 0 : 1; }
