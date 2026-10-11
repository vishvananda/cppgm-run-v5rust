// Constructors may convert their arguments during direct or list initialization.
struct Wrapper
{
  int (*pointer)(int);
  Wrapper(int (*p)(int)) : pointer(p) {}
  int operator()(int n) const { return pointer(n); }
};
int use(const Wrapper& wrapper) { return wrapper(4); }
int main()
{
  auto closure = [](int n) { return n + 1; };
  Wrapper direct(closure);
  Wrapper direct_list{closure};
  Wrapper copy_list = {closure};
  return use(direct) == 5 && use(direct_list) == 5 && use(copy_list) == 5 &&
         use(+closure) == 5 && use(Wrapper(closure)) == 5 &&
         use({closure}) == 5 ? 0 : 1;
}
