// VALIDATION: compile-pass
// A compatible forward declaration may follow a function-template definition.

template<class T>
T identity(T value)
{
  return value;
}

template<class U>
U identity(U value);

int main()
{
  return identity(3) - 3;
}
