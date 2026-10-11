// N3485 7.2p1, 5.19p2, 7.1.5: an enumerator initializer may call a constexpr
// function (an integral constant expression); expect exit 0
constexpr int h(int) { return 1; }
enum { m = h(0) };
int main() { return m == 1 ? 0 : 1; }
