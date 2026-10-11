// VALIDATION: compile-pass
// N3485 focus: 14.1 [temp.param]

template<bool, class T = void>
struct enable_if
{
};

int main()
{
  return 0;
}
