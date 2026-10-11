// VALIDATION: compile-pass
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{static int n;
};
template<class T>struct X<T*>{static int n;
};
template<class T>int X<T>::n=3;
template<class U>int X<U*>::n=7;
int main(){return X<int>::n!=3||X<int*>::n!=7;
}
