// N3485 14.5.3p5: a parameter pack named in a pattern must be expanded;
// typename T::nope* with pack T is unexpanded; must be rejected
template<class... T> char f(typename T::nope*, int);
int main() { return 0; }
