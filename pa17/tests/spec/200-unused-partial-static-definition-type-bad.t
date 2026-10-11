// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{static int n;
};
template<class T>struct X<T*>{static T n;
};
template<class U>U* X<U*>::n;
int main(){}
