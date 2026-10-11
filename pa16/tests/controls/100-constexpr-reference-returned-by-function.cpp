// N3485 5.19p2-p3, 7.1.5p9: a constexpr reference initialized by a constexpr
// function returning one of its reference parameters bound to static objects
// is a reference constant expression; expect exit 0
constexpr const int& larger(const int& a, const int& b) { return a < b ? b : a; }
constexpr int limits[] = { 3, 9 };
constexpr const int& top = larger(limits[0], limits[1]);
int main() { return top == 9 ? 0 : 1; }
