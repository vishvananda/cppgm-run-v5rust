// N3485 13.4p1, 14.8.1p3 (+ 5p? discarded-value): naming a function template
// without a target type (`f;` or `&f;`) is ill-formed; must be rejected
template<class T> int f(T) { return 0; }
int main() { f; return 0; }
