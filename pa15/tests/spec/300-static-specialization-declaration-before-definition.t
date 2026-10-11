// VALIDATION: compile-pass
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{static int n;
};
template<class T>int X<T>::n=3;
template<>int X<int>::n;
template<>int X<int>::n=7;
int main(){return X<char>::n!=3||X<int>::n!=7;
}
