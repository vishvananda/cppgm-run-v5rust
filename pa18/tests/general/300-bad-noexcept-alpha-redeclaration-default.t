// VALIDATION: compile-fail
// Alpha-renaming does not make these dependent exception specifications name
// different templates, so the repeated template default is ill-formed.

template<class T = int>
int inspect(T value = 7) noexcept(sizeof(T) != 0);

template<class U = int>
int inspect(U value) noexcept(sizeof(U) != 0)
{
  return (int)value;
}

int main()
{
  return inspect<int>();
}
