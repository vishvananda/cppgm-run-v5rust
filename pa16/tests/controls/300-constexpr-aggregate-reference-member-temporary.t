// N3485 12.2p5, 5.19p3: in constexpr R rr = { 5, 6 } the temporary bound to
// the reference member has static storage duration; rr.r reads 5;
// expect exit 0
struct R { const int& r; int x; };
constexpr R rr = { 5, 6 };
int main() { return rr.r == 5 ? 0 : 1; }
