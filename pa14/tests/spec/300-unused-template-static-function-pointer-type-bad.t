// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{static int(*p)(T);
};
template<class U>long(*X<U>::p)(U);
int main(){}
