// N3485 14.8.1p9: deduction extends an explicitly specified pack: f<int>('c',
// 2.0) has T = {int, double}; expect exit 0
template<class... T> int f(T... t) { return (int)sizeof...(T); }
int main() { return f<int>('c', 2.0) == 2 ? 0 : 1; }
