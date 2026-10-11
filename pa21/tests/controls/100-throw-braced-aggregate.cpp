// N3485 15.1p3/5.2.3p3: throw Error{7} copy-initializes the exception object
// from a braced aggregate prvalue; expect exit 0
struct Error { int code; };
static void thrower() { throw Error{ 7 }; }
int main() { try { thrower(); } catch (Error e) { return e.code == 7 ? 0 : 1; } return 2; }
