int add(int a, int b) { return a + b; }
template<class... T> int sum(T... t) { return add(t...); }
template<class... T> int nested(T... t) { return sum(sum(t...) + t...); }
int main() { return nested(1, 2) - 9; }
