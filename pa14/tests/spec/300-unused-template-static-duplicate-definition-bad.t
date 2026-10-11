// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{static int n;
};
template<class T>int X<T>::n;
template<class T>int X<T>::n;
int main(){}
