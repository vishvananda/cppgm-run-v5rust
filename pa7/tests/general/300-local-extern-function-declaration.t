namespace n {
struct S {};
}

int main(int argc, char ** argv)
{
  extern ::n::S * make(int argc, char * argv[]);
  ::n::S * (*p)(int, char *[]) = &make;
  return p(argc, argv) == 0;
}
