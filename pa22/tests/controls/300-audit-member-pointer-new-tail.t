// expect exit 0
struct S { int x; };
struct H { int S::*p; };
int run(int n) {
    H *constant = new H[6]{{&S::x}};
    H *dynamic = new H[n]{{&S::x}};
    int result = constant[5].p != nullptr || dynamic[n-1].p != nullptr;
    delete[] constant;
    delete[] dynamic;
    return result;
}
int main() { return run(6); }
