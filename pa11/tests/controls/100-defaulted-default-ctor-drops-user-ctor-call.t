// N3485 8.5p16/13.3.1.3: D d(1500) calls D(long) even though D() is defaulted
// (std::chrono::milliseconds d(1500)); expect exit 0
struct D { long r; D() = default; D(long x) : r(x) {} };
int main() { D d(1500); return d.r == 1500 ? 0 : 1; }
