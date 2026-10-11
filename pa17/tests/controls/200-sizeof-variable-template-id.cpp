// N3485 5.3.3p1, 14.2 + PA17 variable templates: sizeof(v<int>) names a
// variable template specialization (an expression), not a type; expect exit 0
template<class T> long v = 1;
int main() { return sizeof(v<int>) == sizeof(long) ? 0 : 1; }
