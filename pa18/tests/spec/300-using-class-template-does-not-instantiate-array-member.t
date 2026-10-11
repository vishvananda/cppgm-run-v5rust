// VALIDATION: compile-pass
// N3485 focus: 14.7.1 [temp.inst], 7.1.3 [dcl.typedef]

template<class T>
struct lazy_box {
  T value;
};

using namespace_alias = lazy_box<int[]>;

int main()
{
  using local_alias = lazy_box<int[]>;
  (void)sizeof(namespace_alias *);
  (void)sizeof(local_alias *);
  return 0;
}
