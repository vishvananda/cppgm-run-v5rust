// N3485 5.1.2p18: inside a non-mutable [=] lambda, decltype((x)) for an
// automatic x is const float& as if x were a by-copy capture (even when x is
// not odr-used); expect exit 0
int k(float&) { return 1; }
int k(const float&) { return 2; }
int main() { float x = 1; return [=] { decltype(x) y1 = 3; decltype((x)) y2 = y1; return k(y2); }() == 2 ? 0 : 1; }
