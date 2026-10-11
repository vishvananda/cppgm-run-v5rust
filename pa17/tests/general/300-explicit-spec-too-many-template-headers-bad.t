// N3485 14.7.3p15 (C++11 14.7.3p17): a member of S<int> is explicitly
// specialized under one template<> per enclosing class template; two are too
// many; expect reject
template<class T> struct S { int f(); };
template<> template<> int S<int>::f() { return 0; }
int main() { return 0; }
