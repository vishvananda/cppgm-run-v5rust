// VALIDATION: compile-pass
// N3485 focus: 14.8.1 [temp.arg.explicit]

template<class A, class B>
struct pair {};

template<class P, class I, class O, int = 0>
pair<I, O> g(I, int, O);

int main()
{
  return 0;
}
