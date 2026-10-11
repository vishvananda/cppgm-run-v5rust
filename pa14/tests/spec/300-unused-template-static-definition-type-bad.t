// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{static T n;
};
template<class T>T* X<T>::n;
int main(){}
