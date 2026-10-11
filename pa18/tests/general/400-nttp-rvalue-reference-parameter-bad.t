// N3485 14.1p4: a non-type template-parameter shall not have rvalue reference
// type; must be rejected
int x;
template<int&& R> struct P {};
int main() { return 0; }
