// N3485 5.2.2p7: an argument of trivially copyable class type may be passed to
// an ellipsis parameter; expect exit 0
int kind(...) { return 2; }
struct Plain { };
int main() { Plain p; return kind(p) == 2 ? 0 : 1; }
