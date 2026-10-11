// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{};
template<class U>int X<U>::n;
int main(){}
