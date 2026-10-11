// VALIDATION: compile-pass
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{static int(*p)(T);
};
int f(int n){return n+3;
}template<class U>int(*X<U>::p)(U)=f;
int main(){return X<int>::p(4)!=7;
}
