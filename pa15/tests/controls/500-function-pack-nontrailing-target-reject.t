// N3485 14.8.2.2, 14.8.2.5p10: a non-trailing function parameter pack does
// not deduce an address target by ignoring its function's arity; expect reject
template<class... T> int f(T..., int) { return 0; }
int (*target)(int) = &f;
int main() { return target(1); }
