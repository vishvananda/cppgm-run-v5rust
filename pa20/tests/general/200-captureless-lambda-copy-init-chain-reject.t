// Copy initialization cannot chain the closure conversion and a constructor.
struct Wrapper { Wrapper(int (*)(int)) {} };
int main()
{
  Wrapper value = [](int n) { return n + 1; };
  return 0;
}
