// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
namespace N{using A=int;
}template<class T>struct X{static N::A n;
};
template<class U>long X<U>::n;
int main(){}
