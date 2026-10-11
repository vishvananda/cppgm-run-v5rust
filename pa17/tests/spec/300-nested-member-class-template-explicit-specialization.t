// VALIDATION: compile-pass
// N3485 focus: 14.7.3 [temp.expl.spec] specialization of a member class template

template<class> struct A;
template<> struct A<int> { template<class> struct B; };
template<> struct A<int>::B<int> {};

int main() {}
