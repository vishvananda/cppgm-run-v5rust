// N3485 15.3p2, 15.5.1p1: in a noexcept function, an exception from an array
// element constructor inside a try block is caught by that try's handler, not
// std::terminate; expect exit 0
struct U { U() { throw 5; } ~U() {} };
static int f() noexcept { try { U v[10]; return 1; } catch (...) { return 0; } }
int main() { return f(); }
