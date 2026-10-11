// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
template<class T>struct X{static int n;
};
template<>int X<int>::n=3;
template<>int X<int>::n=7;
int main(){return 0;
}
