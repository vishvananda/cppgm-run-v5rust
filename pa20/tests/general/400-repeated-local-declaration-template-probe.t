template<int = 0> struct declared {
  static int const cmp2 = 0;
  friend void operator>(int, declared const&) {}
};
struct undeclared { declared<> value[2]; };
template<int> struct resolve;
template<> struct resolve<sizeof(declared<>)> { static int const cmp1 = 0; };
template<> struct resolve<sizeof(undeclared)> {
  template<int> struct cmp1 { static int const cmp2 = 0; };
};
extern undeclared local_args;
int main() {
  declared<resolve<sizeof(local_args)>::cmp1<0>::cmp2> local_args;
  { declared<resolve<sizeof(local_args)>::cmp1<0>::cmp2> local_args; }
}
