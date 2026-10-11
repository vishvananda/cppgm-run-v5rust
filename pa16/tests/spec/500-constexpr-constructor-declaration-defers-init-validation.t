// N3485 focus: 7.1.5 [dcl.constexpr]

struct deferred_object
{
  int value;

  constexpr deferred_object();
};

int main()
{
  return 0;
}
