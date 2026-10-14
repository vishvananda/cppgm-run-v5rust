// N3485 15.3p13-15/6.6.3: a return in the body of a destructor's
// function-try-block still destroys the members; host -c must compile;
// expect exit 0
static int mode = 1, dtors;
struct Mem { ~Mem() { ++dtors; } };
struct D { Mem m; ~D() try { if (mode) return; } catch (...) { } };
int main() { { D d; } return dtors == 1 ? 0 : 1; }
