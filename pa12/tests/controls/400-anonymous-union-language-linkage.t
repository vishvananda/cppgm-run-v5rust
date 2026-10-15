// expect exit 0
// The hidden union object keeps C++ object identity inside a C linkage block.
extern "C" {
static union { int value = 7; long other; };
}
int main() { return value == 7 ? 0 : 1; }
