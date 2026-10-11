// VALIDATION: compile-pass
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{static T* n;
};
template<class U>U* X<U>::n=0;
int main(){return X<int>::n!=0;
}
