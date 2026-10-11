// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{using A=T;
static A n;
};
template<class U>U* X<U>::n;
int main(){}
