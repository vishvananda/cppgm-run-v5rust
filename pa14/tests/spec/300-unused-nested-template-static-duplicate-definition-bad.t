// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{struct Y{static int n;
};
};
template<class U>int X<U>::Y::n;
template<class V>int X<V>::Y::n;
int main(){}
