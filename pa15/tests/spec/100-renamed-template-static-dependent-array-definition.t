// VALIDATION: compile-pass
// Static-member declaration legality is checked independently of storage demand.
template<int N>struct X{static int n[N];
};
template<int M>int X<M>::n[M]={};
int main(){return sizeof(X<3>::n)!=3*sizeof(int);
}
