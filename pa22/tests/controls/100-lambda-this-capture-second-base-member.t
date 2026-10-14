// N3485 5.1.2p7-p8, 9.3.1p3: inside a [this] lambda, unqualified w names
// this->w of the second (non-primary) base B; the this pointer must be
// adjusted; expect exit 0
struct B0 { int z; };
struct B { int w; };
struct A : B0, B { int y; int r() { auto f = [this]() -> int { return w; }; return f(); } };
int main() { A a; a.z = 0; a.w = 1; a.y = 2; return a.r() == 1 ? 0 : 1; }
