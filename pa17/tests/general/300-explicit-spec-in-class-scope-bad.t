// N3485 14.7.3p2: an explicit specialization shall be declared in a namespace
// enclosing the template, not in class scope (g++ rejects; clang accepts per
// CWG 727); expect reject
struct S { template<class T> int f(T) { return 1; } template<> int f<int>(int) { return 2; } };
int main() { return 0; }
