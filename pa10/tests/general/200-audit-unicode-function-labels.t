// C++ LowIR labels sanitize UTF-8 bytes; ABI object names keep those bytes.
int café() { return 3; }
int caf__() { return 5; }
namespace N {
int café(int x) { return x + 7; }
int café(double) { return 11; }
int alias() { int café(int); return café(1); }
}
extern "C" int ordinary() { return 13; }
int main() {
    return café() == 3 && caf__() == 5 && N::alias() == 8
        && N::café(0.0) == 11 && ordinary() == 13 ? 0 : 1;
}
