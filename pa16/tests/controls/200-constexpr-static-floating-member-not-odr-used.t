// N3485 3.2p2, 9.4.2p3: a constexpr static data member of floating type read
// by value is not odr-used, so no namespace-scope definition is needed;
// expect exit 0
struct K { static constexpr double d = 1.5; };
int main() { double a = K::d; return a == 1.5 ? 0 : 1; }
