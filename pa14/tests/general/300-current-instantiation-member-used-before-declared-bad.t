// N3485 14.6.2.1p4, 3.4.1p7: S<T>::I names a member of the current
// instantiation not yet declared at that point; expect reject
template<class T> struct S { S<T>::I x; typedef int I; };
int main() { return 0; }
